#![allow(dead_code)] // The private observation slice is consumed by the later persistence task.

use std::{
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
};

use git2::Repository;
use rusqlite::{Connection, OpenFlags};

use crate::canonical;

use super::{
    DiscoveryContextKind, MAX_DOCUMENT_DIRECTORY_DEPTH, MAX_DOCUMENT_DIRECTORY_ENTRIES,
    MAX_MANAGED_DIRECTORY_ENTRIES, REGISTRY_BUSY_TIMEOUT, RegistryConnectionPhase, RepositoryError,
};

#[derive(Debug)]
enum RootConfiguration {
    Missing,
    Valid(canonical::RepositoryConfig),
    Invalid(canonical::ValidationProblem),
}

#[derive(Debug)]
struct RootHeadObservation {
    branch: Option<String>,
    oid: Option<git2::Oid>,
}

#[derive(Debug)]
struct RootContextObservation {
    kind: DiscoveryContextKind,
    branch: Option<String>,
}

#[derive(Debug)]
enum RootObservationProblem {
    Configuration(canonical::ValidationProblem),
    Branch { message: String },
    Source { path: PathBuf, message: String },
    Context { path: PathBuf, message: String },
}

#[derive(Debug)]
struct RootObservation {
    configuration: RootConfiguration,
    context: RootContextObservation,
    head: RootHeadObservation,
    sources: Vec<(PathBuf, String)>,
    validation: canonical::ValidatedContext,
    problems: Vec<RootObservationProblem>,
    active_contexts: Vec<ActiveContextObservation>,
}

#[derive(Debug)]
struct ActiveContextObservation {
    context: RootContextObservation,
    head: RootHeadObservation,
    sources: Vec<(PathBuf, String)>,
    validation: canonical::ValidatedContext,
    problems: Vec<RootObservationProblem>,
}

fn observe_root(repository: &Repository, root: &Path) -> RootObservation {
    let mut problems = Vec::new();
    let configuration = observe_configuration(root, &mut problems);
    let head = observe_head(repository, &mut problems);
    let kind = match (&configuration, &head.branch) {
        (RootConfiguration::Valid(configuration), Some(branch))
            if branch == &configuration.primary_branch =>
        {
            DiscoveryContextKind::Primary
        }
        (RootConfiguration::Valid(configuration), Some(branch)) => {
            problems.push(RootObservationProblem::Branch {
                message: format!(
                    "the configured primary branch {} is not checked out (found {branch})",
                    configuration.primary_branch
                ),
            });
            DiscoveryContextKind::Unverified
        }
        (RootConfiguration::Valid(_), None) => {
            problems.push(RootObservationProblem::Branch {
                message: "the repository root does not have a checked-out local branch".to_owned(),
            });
            DiscoveryContextKind::Unverified
        }
        (RootConfiguration::Missing | RootConfiguration::Invalid(_), _) => {
            DiscoveryContextKind::Unverified
        }
    };
    let mut sources = Vec::new();
    collect_root_sources(root, &mut sources, &mut problems);
    let validation = canonical::validate_context(sources.clone());
    let active_contexts = match configuration {
        RootConfiguration::Valid(_) => observe_active_contexts(repository, root, &mut problems),
        RootConfiguration::Missing | RootConfiguration::Invalid(_) => Vec::new(),
    };

    RootObservation {
        configuration,
        context: RootContextObservation {
            kind,
            branch: head.branch.clone(),
        },
        head,
        sources,
        validation,
        problems,
        active_contexts,
    }
}

fn observe_active_contexts(
    root_repository: &Repository,
    root: &Path,
    problems: &mut Vec<RootObservationProblem>,
) -> Vec<ActiveContextObservation> {
    let base = root.join(".manyhands/worktrees");
    if !directory_exists(&base, problems) {
        return Vec::new();
    }
    let mut entries = 0;
    let mut contexts = Vec::new();
    for entry in read_managed_directory(&base, &mut entries, problems) {
        let path = entry.path();
        let Ok(file_type) = entry.file_type() else {
            problems.push(context_problem(
                path,
                "the worktree entry type cannot be read",
            ));
            continue;
        };
        if file_type.is_symlink() || !file_type.is_dir() {
            continue;
        }
        let Some(path_id) = entry
            .file_name()
            .to_str()
            .and_then(|name| name.parse::<canonical::ItemId>().ok())
        else {
            continue;
        };
        let expected_path = root.join(".manyhands/worktrees").join(path_id.to_string());
        let Ok(expected_path) = expected_path.canonicalize() else {
            problems.push(context_problem(
                path,
                "the deterministic worktree path cannot be canonicalized",
            ));
            continue;
        };
        let Ok(registered_worktree) = root_repository.find_worktree(&path_id.to_string()) else {
            problems.push(context_problem(
                path,
                "the worktree is not registered by the repository root",
            ));
            continue;
        };
        let Ok(registered_path) = registered_worktree.path().canonicalize() else {
            problems.push(context_problem(
                path,
                "the registered worktree path cannot be canonicalized",
            ));
            continue;
        };
        if registered_path != expected_path {
            problems.push(context_problem(
                path,
                "the registered worktree path does not match the deterministic path",
            ));
            continue;
        }
        let Ok(repository) = Repository::open(&registered_path) else {
            problems.push(context_problem(
                path,
                "the registered worktree repository cannot be opened",
            ));
            continue;
        };
        if repository
            .workdir()
            .and_then(|workdir| workdir.canonicalize().ok())
            .as_deref()
            != Some(expected_path.as_path())
        {
            problems.push(context_problem(
                path,
                "the registered worktree has an unexpected canonical workdir",
            ));
            continue;
        }
        let mut context_problems = Vec::new();
        let head = observe_head(&repository, &mut context_problems);
        let Some((kind, branch_id)) = head.branch.as_deref().and_then(parse_authoring_branch)
        else {
            problems.push(context_problem(
                path,
                "the worktree does not have a conforming authoring branch",
            ));
            continue;
        };
        if branch_id != path_id {
            problems.push(context_problem(
                path,
                "the worktree path does not match its checked-out authoring branch",
            ));
            continue;
        }
        let mut sources = Vec::new();
        collect_root_sources(&expected_path, &mut sources, &mut context_problems);
        let validation = canonical::validate_context(sources.clone());
        if !validation
            .items
            .iter()
            .any(|item| authoring_item_matches(item, kind, &path_id))
        {
            problems.push(context_problem(
                path,
                "the authoring branch's identified item is missing or invalid",
            ));
            continue;
        }
        contexts.push(ActiveContextObservation {
            context: RootContextObservation {
                kind: DiscoveryContextKind::Active,
                branch: head.branch.clone(),
            },
            head,
            sources,
            validation,
            problems: context_problems,
        });
    }
    contexts.sort_by(|left, right| left.context.branch.cmp(&right.context.branch));
    contexts
}

fn parse_authoring_branch(
    branch: &str,
) -> Option<(crate::repository::AuthoringKind, canonical::ItemId)> {
    let mut segments = branch.strip_prefix("manyhands/")?.split('/');
    let kind = match segments.next()? {
        "document" => crate::repository::AuthoringKind::Document,
        "ticket" => crate::repository::AuthoringKind::Ticket,
        _ => return None,
    };
    let item_id = segments.next()?.parse().ok()?;
    if segments.next().is_some() {
        return None;
    }
    Some((kind, item_id))
}

fn authoring_item_matches(
    item: &canonical::CanonicalItem,
    kind: crate::repository::AuthoringKind,
    item_id: &canonical::ItemId,
) -> bool {
    match (item, kind) {
        (
            canonical::CanonicalItem::Document(document),
            crate::repository::AuthoringKind::Document,
        ) => document.id == *item_id,
        (canonical::CanonicalItem::Ticket(ticket), crate::repository::AuthoringKind::Ticket) => {
            ticket.id == *item_id
        }
        _ => false,
    }
}

fn observe_configuration(
    root: &Path,
    problems: &mut Vec<RootObservationProblem>,
) -> RootConfiguration {
    let path = root.join(canonical::CONFIG_PATH);
    let parent = root.join(".manyhands");
    match fs::symlink_metadata(&parent) {
        Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => {}
        Ok(_) => return invalid_configuration(problems, "the configuration directory is unsafe"),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return missing_configuration(problems);
        }
        Err(error) => {
            return invalid_configuration(
                problems,
                format!("the configuration directory cannot be inspected: {error}"),
            );
        }
    }
    match fs::symlink_metadata(&path) {
        Ok(metadata) if metadata.file_type().is_file() && !metadata.file_type().is_symlink() => {}
        Ok(_) => return invalid_configuration(problems, "the configuration file is unsafe"),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return missing_configuration(problems);
        }
        Err(error) => {
            return invalid_configuration(
                problems,
                format!("the repository configuration cannot be inspected: {error}"),
            );
        }
    }
    match fs::read_to_string(&path) {
        Ok(source) => match canonical::parse_repository_config(&source) {
            Ok(configuration) => RootConfiguration::Valid(configuration),
            Err(problem) => invalid_configuration_problem(problems, problem),
        },
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            missing_configuration(problems)
        }
        Err(error) => invalid_configuration(
            problems,
            format!("the repository configuration cannot be read: {error}"),
        ),
    }
}

fn missing_configuration(problems: &mut Vec<RootObservationProblem>) -> RootConfiguration {
    let problem = configuration_problem(
        canonical::ValidationCode::MissingField,
        "the repository configuration is missing",
    );
    problems.push(RootObservationProblem::Configuration(problem));
    RootConfiguration::Missing
}

fn invalid_configuration(
    problems: &mut Vec<RootObservationProblem>,
    message: impl Into<String>,
) -> RootConfiguration {
    let problem = configuration_problem(canonical::ValidationCode::MalformedConfiguration, message);
    invalid_configuration_problem(problems, problem)
}

fn invalid_configuration_problem(
    problems: &mut Vec<RootObservationProblem>,
    problem: canonical::ValidationProblem,
) -> RootConfiguration {
    problems.push(RootObservationProblem::Configuration(problem.clone()));
    RootConfiguration::Invalid(problem)
}

fn configuration_problem(
    code: canonical::ValidationCode,
    message: impl Into<String>,
) -> canonical::ValidationProblem {
    canonical::ValidationProblem {
        path: PathBuf::from(canonical::CONFIG_PATH),
        code,
        message: message.into(),
    }
}

fn observe_head(
    repository: &Repository,
    problems: &mut Vec<RootObservationProblem>,
) -> RootHeadObservation {
    match repository.find_reference("HEAD") {
        Ok(head) => RootHeadObservation {
            branch: head
                .symbolic_target()
                .and_then(|name| name.strip_prefix("refs/heads/"))
                .map(str::to_owned),
            oid: head.resolve().ok().and_then(|head| head.target()),
        },
        Err(error) => {
            problems.push(RootObservationProblem::Branch {
                message: format!("the repository HEAD cannot be observed: {error}"),
            });
            RootHeadObservation {
                branch: None,
                oid: None,
            }
        }
    }
}

fn collect_root_sources(
    root: &Path,
    sources: &mut Vec<(PathBuf, String)>,
    problems: &mut Vec<RootObservationProblem>,
) {
    collect_documents(root, sources, problems);
    collect_tickets(root, sources, problems);
    collect_comments(root, sources, problems);
    collect_managed_out_of_path_sources(root, sources, problems);
    sources.sort_by(|left, right| left.0.cmp(&right.0));
}

fn collect_managed_out_of_path_sources(
    root: &Path,
    sources: &mut Vec<(PathBuf, String)>,
    problems: &mut Vec<RootObservationProblem>,
) {
    let mut directories = vec![(root.to_owned(), 0)];
    let mut entries = 0;
    while let Some((directory, depth)) = directories.pop() {
        let (directory_entries, overflowed) =
            read_marker_directory(&directory, &mut entries, problems);
        for entry in directory_entries {
            let path = entry.path();
            let Ok(file_type) = entry.file_type() else {
                problems.push(source_problem(
                    path,
                    "the directory entry type cannot be read",
                ));
                continue;
            };
            if file_type.is_symlink() || is_canonical_source_directory(root, &path) {
                continue;
            }
            if file_type.is_dir() {
                if is_excluded_directory(&path) {
                    continue;
                }
                if depth >= MAX_DOCUMENT_DIRECTORY_DEPTH {
                    problems.push(source_problem(
                        path,
                        "the repository directory exceeds the bounded managed marker traversal limit",
                    ));
                } else {
                    directories.push((path, depth + 1));
                }
            } else if file_type.is_file()
                && path.extension() == Some(OsStr::new("md"))
                && file_declares_managed_marker(&path)
            {
                collect_source(root, &path, sources, problems);
            }
        }
        if overflowed {
            return;
        }
    }
}

fn is_canonical_source_directory(root: &Path, path: &Path) -> bool {
    path == root.join("docs")
        || path == root.join(".manyhands/tickets")
        || path == root.join(".manyhands/comments")
        || path == root.join(".manyhands/worktrees")
}

fn file_declares_managed_marker(path: &Path) -> bool {
    let Ok(source) = fs::read_to_string(path) else {
        return false;
    };
    let content_start = if source.starts_with("---\n") {
        4
    } else if source.starts_with("---\r\n") {
        5
    } else {
        return false;
    };
    let mut line_start = content_start;
    while line_start < source.len() {
        let remaining = &source[line_start..];
        let Some(newline) = remaining.find('\n') else {
            return remaining == "---"
                && yaml_declares_managed_marker(&source[content_start..line_start]);
        };
        let line_end = line_start + newline;
        let line = source[line_start..line_end]
            .strip_suffix('\r')
            .unwrap_or(&source[line_start..line_end]);
        if line == "---" {
            return yaml_declares_managed_marker(&source[content_start..line_start]);
        }
        line_start = line_end + 1;
    }
    false
}

fn yaml_declares_managed_marker(front_matter: &str) -> bool {
    serde_yaml::from_str::<serde_yaml::Value>(front_matter)
        .ok()
        .and_then(|value| value.as_mapping().cloned())
        .and_then(|values| {
            values
                .get(serde_yaml::Value::String("manyhands_managed".into()))
                .cloned()
        })
        .and_then(|value| value.as_bool())
        == Some(true)
}

fn collect_documents(
    root: &Path,
    sources: &mut Vec<(PathBuf, String)>,
    problems: &mut Vec<RootObservationProblem>,
) {
    let docs = root.join("docs");
    if !directory_exists(&docs, problems) {
        return;
    }
    let mut directories = vec![(docs, 0)];
    let mut entries = 0;
    while let Some((directory, depth)) = directories.pop() {
        let (directory_entries, overflowed) =
            read_document_directory(&directory, &mut entries, problems);
        for entry in directory_entries {
            let path = entry.path();
            let Ok(file_type) = entry.file_type() else {
                problems.push(source_problem(
                    path,
                    "the directory entry type cannot be read",
                ));
                continue;
            };
            if file_type.is_symlink() {
                problems.push(source_problem(
                    path,
                    "symbolic links are not canonical sources",
                ));
            } else if file_type.is_dir() {
                if is_excluded_directory(&path) {
                    continue;
                }
                if depth >= MAX_DOCUMENT_DIRECTORY_DEPTH {
                    problems.push(source_problem(
                        path,
                        "the docs directory exceeds the bounded canonical source traversal limit",
                    ));
                } else {
                    directories.push((path, depth + 1));
                }
            } else if file_type.is_file() && path.extension() == Some(OsStr::new("md")) {
                collect_source(root, &path, sources, problems);
            }
        }
        if overflowed {
            return;
        }
    }
}

fn collect_tickets(
    root: &Path,
    sources: &mut Vec<(PathBuf, String)>,
    problems: &mut Vec<RootObservationProblem>,
) {
    let directory = root.join(".manyhands/tickets");
    if !directory_exists(&directory, problems) {
        return;
    }
    let mut entries = 0;
    for entry in read_managed_directory(&directory, &mut entries, problems) {
        let path = entry.path();
        let Ok(file_type) = entry.file_type() else {
            problems.push(source_problem(
                path,
                "the directory entry type cannot be read",
            ));
            continue;
        };
        if file_type.is_symlink() {
            problems.push(source_problem(
                path,
                "symbolic links are not canonical sources",
            ));
        } else if file_type.is_dir() && !is_excluded_directory(&path) {
            collect_source(root, &path.join("ticket.md"), sources, problems);
        }
    }
}

fn collect_comments(
    root: &Path,
    sources: &mut Vec<(PathBuf, String)>,
    problems: &mut Vec<RootObservationProblem>,
) {
    let directory = root.join(".manyhands/comments");
    if !directory_exists(&directory, problems) {
        return;
    }
    let mut entries = 0;
    for entry in read_managed_directory(&directory, &mut entries, problems) {
        let path = entry.path();
        let Ok(file_type) = entry.file_type() else {
            problems.push(source_problem(
                path,
                "the directory entry type cannot be read",
            ));
            continue;
        };
        if file_type.is_symlink() {
            problems.push(source_problem(
                path,
                "symbolic links are not canonical sources",
            ));
        } else if file_type.is_dir() && !is_excluded_directory(&path) {
            for entry in read_managed_directory(&path, &mut entries, problems) {
                let path = entry.path();
                let Ok(file_type) = entry.file_type() else {
                    problems.push(source_problem(
                        path,
                        "the directory entry type cannot be read",
                    ));
                    continue;
                };
                if file_type.is_symlink() {
                    problems.push(source_problem(
                        path,
                        "symbolic links are not canonical sources",
                    ));
                } else if file_type.is_file() && path.extension() == Some(OsStr::new("md")) {
                    collect_source(root, &path, sources, problems);
                }
            }
        }
    }
}

fn is_excluded_directory(path: &Path) -> bool {
    path.file_name() == Some(OsStr::new(".git"))
}

fn read_managed_directory(
    path: &Path,
    entries: &mut usize,
    problems: &mut Vec<RootObservationProblem>,
) -> Vec<fs::DirEntry> {
    let mut directory = match fs::read_dir(path) {
        Ok(directory) => directory,
        Err(error) => {
            problems.push(source_problem(
                path,
                format!("the directory cannot be read: {error}"),
            ));
            return Vec::new();
        }
    };
    let mut collected = Vec::new();
    loop {
        let Some(entry) = directory.next() else {
            break;
        };
        if *entries >= MAX_MANAGED_DIRECTORY_ENTRIES {
            problems.push(source_problem(
                path,
                format!(
                    "the managed directory exceeds the bounded canonical source traversal limit; remove entries so it contains at most {MAX_MANAGED_DIRECTORY_ENTRIES} entries"
                ),
            ));
            break;
        }
        *entries += 1;
        match entry {
            Ok(entry) => collected.push(entry),
            Err(error) => problems.push(source_problem(
                path,
                format!("a directory entry cannot be read: {error}"),
            )),
        }
    }
    collected.sort_by_key(|entry| entry.file_name());
    collected
}

fn directory_exists(path: &Path, problems: &mut Vec<RootObservationProblem>) -> bool {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            problems.push(source_problem(
                path,
                "symbolic links are not safe canonical directories",
            ));
            false
        }
        Ok(metadata) if metadata.is_dir() => true,
        Ok(_) => {
            problems.push(source_problem(
                path,
                "the canonical source directory is not a directory",
            ));
            false
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
        Err(error) => {
            problems.push(source_problem(
                path,
                format!("the directory cannot be read: {error}"),
            ));
            false
        }
    }
}

fn read_document_directory(
    path: &Path,
    entries: &mut usize,
    problems: &mut Vec<RootObservationProblem>,
) -> (Vec<fs::DirEntry>, bool) {
    let mut directory = match fs::read_dir(path) {
        Ok(directory) => directory,
        Err(error) => {
            problems.push(source_problem(
                path,
                format!("the directory cannot be read: {error}"),
            ));
            return (Vec::new(), false);
        }
    };
    let mut collected = Vec::new();
    let mut overflowed = false;
    loop {
        let Some(entry) = directory.next() else {
            break;
        };
        if *entries >= MAX_DOCUMENT_DIRECTORY_ENTRIES {
            problems.push(source_problem(
                path,
                "the docs directory exceeds the bounded canonical source traversal limit",
            ));
            overflowed = true;
            break;
        }
        *entries += 1;
        match entry {
            Ok(entry) => collected.push(entry),
            Err(error) => problems.push(source_problem(
                path,
                format!("a directory entry cannot be read: {error}"),
            )),
        }
    }
    collected.sort_by_key(|entry| entry.file_name());
    (collected, overflowed)
}

fn read_marker_directory(
    path: &Path,
    entries: &mut usize,
    problems: &mut Vec<RootObservationProblem>,
) -> (Vec<fs::DirEntry>, bool) {
    read_document_directory(path, entries, problems)
}

fn collect_source(
    root: &Path,
    path: &Path,
    sources: &mut Vec<(PathBuf, String)>,
    problems: &mut Vec<RootObservationProblem>,
) {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_file() && !metadata.file_type().is_symlink() => {}
        Ok(_) => {
            problems.push(source_problem(
                path,
                "the canonical source must be a regular file and not a symbolic link",
            ));
            return;
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return,
        Err(error) => {
            problems.push(source_problem(
                path,
                format!("the source cannot be inspected: {error}"),
            ));
            return;
        }
    }
    let relative = match path.strip_prefix(root) {
        Ok(path) => path.to_owned(),
        Err(_) => {
            problems.push(source_problem(
                path,
                "the source is outside the repository root",
            ));
            return;
        }
    };
    match fs::read_to_string(path) {
        Ok(source) => sources.push((relative, source)),
        Err(error) => problems.push(source_problem(
            path,
            format!("the canonical source cannot be read as UTF-8: {error}"),
        )),
    }
}

fn source_problem(path: impl Into<PathBuf>, message: impl Into<String>) -> RootObservationProblem {
    RootObservationProblem::Source {
        path: path.into(),
        message: message.into(),
    }
}

fn context_problem(path: impl Into<PathBuf>, message: impl Into<String>) -> RootObservationProblem {
    RootObservationProblem::Context {
        path: path.into(),
        message: message.into(),
    }
}

pub(super) fn open_registry(
    registry_path: &Path,
    observer: &mut impl FnMut(RegistryConnectionPhase),
) -> Result<Connection, RepositoryError> {
    let connection = Connection::open(registry_path).map_err(RepositoryError::sqlite)?;
    connection
        .busy_timeout(REGISTRY_BUSY_TIMEOUT)
        .map_err(RepositoryError::sqlite)?;
    connection
        .pragma_update(None, "foreign_keys", "ON")
        .map_err(RepositoryError::sqlite)?;
    observer(RegistryConnectionPhase::BeforeWal);
    connection
        .pragma_update(None, "journal_mode", "WAL")
        .map_err(RepositoryError::sqlite)?;
    observer(RegistryConnectionPhase::AfterWal);

    Ok(connection)
}

pub(super) fn open_registry_read_only(registry_path: &Path) -> Result<Connection, RepositoryError> {
    let connection = Connection::open_with_flags(registry_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(RepositoryError::sqlite)?;
    connection
        .busy_timeout(REGISTRY_BUSY_TIMEOUT)
        .map_err(RepositoryError::sqlite)?;
    Ok(connection)
}

pub(super) fn migrate_registry(connection: &mut Connection) -> Result<(), RepositoryError> {
    let transaction = connection.transaction().map_err(RepositoryError::sqlite)?;
    let repositories_exist = transaction
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'repositories')",
            [],
            |row| row.get::<_, bool>(0),
        )
        .map_err(RepositoryError::sqlite)?;
    let config_blob_oid_is_required = if repositories_exist {
        transaction
            .query_row(
                "SELECT \"notnull\" FROM pragma_table_info('repositories') WHERE name = 'config_blob_oid'",
                [],
                |row| row.get::<_, bool>(0),
            )
            .map_err(RepositoryError::sqlite)?
    } else {
        false
    };

    if config_blob_oid_is_required {
        transaction
            .execute_batch(
                "ALTER TABLE repositories RENAME TO repositories_cycle_02;
                 CREATE TABLE repositories (
                    id INTEGER PRIMARY KEY,
                    root_path TEXT NOT NULL UNIQUE,
                    enabled_at INTEGER NOT NULL,
                    accessibility TEXT NOT NULL,
                    config_blob_oid TEXT,
                    refresh_required INTEGER NOT NULL CHECK (refresh_required IN (0, 1))
                 );
                 INSERT INTO repositories (
                    id, root_path, enabled_at, accessibility, config_blob_oid, refresh_required
                 ) SELECT
                    id, root_path, enabled_at, accessibility, config_blob_oid, refresh_required
                 FROM repositories_cycle_02;
                 DROP TABLE repositories_cycle_02;",
            )
            .map_err(RepositoryError::sqlite)?;
    } else {
        transaction
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS repositories (
                    id INTEGER PRIMARY KEY,
                    root_path TEXT NOT NULL UNIQUE,
                    enabled_at INTEGER NOT NULL,
                    accessibility TEXT NOT NULL,
                    config_blob_oid TEXT,
                    refresh_required INTEGER NOT NULL CHECK (refresh_required IN (0, 1))
                );",
            )
            .map_err(RepositoryError::sqlite)?;
    }

    transaction
        .execute_batch(
            "CREATE TABLE IF NOT EXISTS contexts (
                id INTEGER PRIMARY KEY,
                repository_id INTEGER NOT NULL REFERENCES repositories(id) ON DELETE CASCADE,
                kind TEXT NOT NULL,
                branch TEXT,
                worktree_path TEXT NOT NULL,
                item_id TEXT,
                head_oid TEXT,
                UNIQUE(repository_id, worktree_path)
            );
            CREATE TABLE IF NOT EXISTS discovered_items (
                id INTEGER PRIMARY KEY,
                context_id INTEGER NOT NULL REFERENCES contexts(id) ON DELETE CASCADE,
                item_id TEXT NOT NULL,
                kind TEXT NOT NULL,
                canonical_path TEXT NOT NULL,
                title TEXT NOT NULL,
                ticket_type TEXT,
                status TEXT,
                project TEXT,
                team TEXT,
                closed_at INTEGER,
                activity_at INTEGER NOT NULL,
                activity_source TEXT NOT NULL,
                UNIQUE(context_id, item_id)
            );
            CREATE TABLE IF NOT EXISTS discovered_comments (
                id INTEGER PRIMARY KEY,
                item_id INTEGER NOT NULL REFERENCES discovered_items(id) ON DELETE CASCADE,
                comment_id TEXT NOT NULL,
                parent_comment_id TEXT,
                canonical_path TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                UNIQUE(item_id, comment_id)
            );
             CREATE TABLE IF NOT EXISTS problems (
                id INTEGER PRIMARY KEY,
                repository_id INTEGER NOT NULL REFERENCES repositories(id) ON DELETE CASCADE,
                context_id INTEGER REFERENCES contexts(id) ON DELETE CASCADE,
                path TEXT,
                code TEXT NOT NULL,
                guidance TEXT NOT NULL,
                observed_at INTEGER NOT NULL
             );
             CREATE TABLE IF NOT EXISTS configuration_observations (
                repository_id INTEGER PRIMARY KEY REFERENCES repositories(id) ON DELETE CASCADE,
                state TEXT NOT NULL,
                primary_branch TEXT,
                publication_remote TEXT,
                invalid_code TEXT,
                guidance TEXT
             );
             CREATE TABLE IF NOT EXISTS index_operations (
                id INTEGER PRIMARY KEY,
                repository_id INTEGER NOT NULL REFERENCES repositories(id) ON DELETE CASCADE,
                operation TEXT NOT NULL,
                observed_at INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS contexts_repository_id_idx
                ON contexts(repository_id, worktree_path);
            CREATE INDEX IF NOT EXISTS discovered_items_context_id_idx
                ON discovered_items(context_id, canonical_path);
            CREATE INDEX IF NOT EXISTS discovered_comments_item_id_idx
                ON discovered_comments(item_id, created_at, comment_id);
            CREATE INDEX IF NOT EXISTS problems_repository_id_idx
                ON problems(repository_id, context_id, observed_at);
            CREATE INDEX IF NOT EXISTS index_operations_repository_id_idx
                ON index_operations(repository_id, observed_at);",
        )
        .map_err(RepositoryError::sqlite)?;
    transaction.commit().map_err(RepositoryError::sqlite)
}

#[cfg(test)]
mod tests {
    #[cfg(unix)]
    use std::os::unix::fs::symlink;
    use std::{fs, path::Path};

    use git2::{Repository, RepositoryInitOptions, Signature, Time, WorktreeAddOptions};

    use super::*;
    use crate::{
        canonical,
        repository::{DiscoveryContextKind, MAX_DOCUMENT_DIRECTORY_DEPTH},
    };

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

    fn write_config(root: &Path, source: &str) {
        let path = root.join(canonical::CONFIG_PATH);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, source).unwrap();
    }

    fn document_source() -> &'static str {
        "---\nmanyhands_managed: true\nmanyhands_kind: document\nid: 01ARZ3NDEKTSV4RRFFQ69G5FAV\ntitle: Fixture document\n---\n"
    }

    fn ticket_source() -> &'static str {
        "---\nmanyhands_managed: true\nmanyhands_kind: ticket\nid: 01ARZ3NDEKTSV4RRFFQ69G5FAW\ntitle: Fixture ticket\ntype: task\nstatus: open\n---\n"
    }

    fn comment_source() -> &'static str {
        "---\nmanyhands_managed: true\nmanyhands_kind: comment\nid: 01ARZ3NDEKTSV4RRFFQ69G5FAX\nitem_id: 01ARZ3NDEKTSV4RRFFQ69G5FAV\ncreated_at: 2026-09-30T12:00:00Z\n---\n"
    }

    fn active_worktree(repository: &Repository, root: &Path, kind: &str, item_id: &str) -> PathBuf {
        active_worktree_at(repository, root, kind, item_id, item_id)
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

    #[test]
    fn observes_a_valid_primary_root_and_its_canonical_document() {
        let (directory, repository) = repository_on_main();
        write_config(
            directory.path(),
            "format_version = 1\nprimary_branch = \"main\"\n",
        );
        fs::create_dir_all(directory.path().join("docs")).unwrap();
        fs::write(directory.path().join("docs/document.md"), document_source()).unwrap();

        let observation = observe_root(&repository, directory.path());

        assert!(matches!(
            observation.configuration,
            RootConfiguration::Valid(ref config) if config.primary_branch == "main"
        ));
        assert_eq!(observation.context.kind, DiscoveryContextKind::Primary);
        assert_eq!(observation.head.branch.as_deref(), Some("main"));
        assert_eq!(observation.sources.len(), 1);
        assert_eq!(observation.validation.items.len(), 1);
        assert!(observation.problems.is_empty());
    }

    #[test]
    fn missing_or_malformed_configuration_is_unverified_but_keeps_documents() {
        for config in [
            None,
            Some("format_version = 2\nprimary_branch = \"main\"\n"),
        ] {
            let (directory, repository) = repository_on_main();
            if let Some(config) = config {
                write_config(directory.path(), config);
            }
            fs::create_dir_all(directory.path().join("docs")).unwrap();
            fs::write(directory.path().join("docs/document.md"), document_source()).unwrap();

            let observation = observe_root(&repository, directory.path());

            assert!(!matches!(
                observation.configuration,
                RootConfiguration::Valid(_)
            ));
            assert_eq!(observation.context.kind, DiscoveryContextKind::Unverified);
            assert_eq!(observation.validation.items.len(), 1);
            assert!(
                observation
                    .problems
                    .iter()
                    .any(|problem| matches!(problem, RootObservationProblem::Configuration(_)))
            );
        }
    }

    #[test]
    fn syntactically_malformed_configuration_is_unverified_but_keeps_documents() {
        let (directory, repository) = repository_on_main();
        write_config(
            directory.path(),
            "format_version = 1\nprimary_branch = [\"main\"\n",
        );
        fs::create_dir_all(directory.path().join("docs")).unwrap();
        fs::write(directory.path().join("docs/document.md"), document_source()).unwrap();

        let observation = observe_root(&repository, directory.path());

        assert!(matches!(
            observation.configuration,
            RootConfiguration::Invalid(ref problem)
                if problem.code == canonical::ValidationCode::MalformedConfiguration
        ));
        assert_eq!(observation.context.kind, DiscoveryContextKind::Unverified);
        assert_eq!(observation.validation.items.len(), 1);
        assert!(
            observation
                .problems
                .iter()
                .any(|problem| matches!(problem, RootObservationProblem::Configuration(_)))
        );
    }

    #[test]
    fn wrong_checked_out_primary_branch_is_unverified() {
        let (directory, repository) = repository_on_main();
        write_config(
            directory.path(),
            "format_version = 1\nprimary_branch = \"other\"\n",
        );

        let observation = observe_root(&repository, directory.path());

        assert_eq!(observation.context.kind, DiscoveryContextKind::Unverified);
        assert!(
            observation
                .problems
                .iter()
                .any(|problem| matches!(problem, RootObservationProblem::Branch { .. }))
        );
    }

    #[test]
    fn reports_marker_only_and_malformed_canonical_content() {
        let (directory, repository) = repository_on_main();
        write_config(
            directory.path(),
            "format_version = 1\nprimary_branch = \"main\"\n",
        );
        fs::create_dir_all(directory.path().join("docs")).unwrap();
        fs::write(
            directory.path().join("docs/marker-only.md"),
            "manyhands_managed: true\n",
        )
        .unwrap();
        fs::write(
            directory.path().join("docs/malformed.md"),
            "---\nnot: [yaml\n---\n",
        )
        .unwrap();

        let observation = observe_root(&repository, directory.path());

        assert_eq!(observation.sources.len(), 2);
        assert_eq!(observation.validation.items.len(), 0);
        assert_eq!(observation.validation.problems.len(), 2);
        assert!(
            observation
                .validation
                .problems
                .iter()
                .any(|problem| problem.code == canonical::ValidationCode::MissingFrontMatter)
        );
        assert!(
            observation
                .validation
                .problems
                .iter()
                .any(|problem| problem.code == canonical::ValidationCode::MalformedFrontMatter)
        );
    }

    #[test]
    fn reports_a_managed_marker_outside_canonical_locations() {
        let (directory, repository) = repository_on_main();
        write_config(
            directory.path(),
            "format_version = 1\nprimary_branch = \"main\"\n",
        );
        let path = directory.path().join("notes/managed.md");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, document_source()).unwrap();

        let observation = observe_root(&repository, directory.path());

        assert!(
            observation
                .validation
                .problems
                .iter()
                .any(|problem| problem.path == Path::new("notes/managed.md"))
        );
    }

    #[test]
    fn detects_a_managed_marker_with_an_end_of_file_closing_delimiter() {
        let (directory, repository) = repository_on_main();
        write_config(
            directory.path(),
            "format_version = 1\nprimary_branch = \"main\"\n",
        );
        let path = directory.path().join("notes/managed.md");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "---\nmanyhands_managed: true\n---").unwrap();

        let observation = observe_root(&repository, directory.path());

        assert!(
            observation
                .validation
                .problems
                .iter()
                .any(|problem| problem.path == Path::new("notes/managed.md"))
        );
    }

    #[test]
    fn ignores_ordinary_markdown_outside_canonical_locations() {
        let (directory, repository) = repository_on_main();
        write_config(
            directory.path(),
            "format_version = 1\nprimary_branch = \"main\"\n",
        );
        fs::write(
            directory.path().join("README.md"),
            "This mentions manyhands_managed: true but is not front matter.\n",
        )
        .unwrap();

        let observation = observe_root(&repository, directory.path());

        assert!(observation.sources.is_empty());
        assert!(observation.validation.problems.is_empty());
    }

    #[test]
    fn ignores_front_matter_without_a_managed_marker_even_when_its_body_mentions_one() {
        let (directory, repository) = repository_on_main();
        write_config(
            directory.path(),
            "format_version = 1\nprimary_branch = \"main\"\n",
        );
        fs::write(
            directory.path().join("README.md"),
            "---\ntitle: Ordinary README\n---\nThe body mentions manyhands_managed: true.\n",
        )
        .unwrap();

        let observation = observe_root(&repository, directory.path());

        assert!(observation.sources.is_empty());
        assert!(observation.validation.problems.is_empty());
    }

    #[test]
    fn observes_valid_document_and_ticket_active_contexts() {
        let (directory, repository) = repository_on_main();
        write_config(
            directory.path(),
            "format_version = 1\nprimary_branch = \"main\"\n",
        );
        let document_id = "01ARZ3NDEKTSV4RRFFQ69G5FAV";
        let ticket_id = "01ARZ3NDEKTSV4RRFFQ69G5FAW";
        let document = active_worktree(&repository, directory.path(), "document", document_id);
        let ticket = active_worktree(&repository, directory.path(), "ticket", ticket_id);
        fs::create_dir_all(document.join("docs")).unwrap();
        fs::write(document.join("docs/document.md"), document_source()).unwrap();
        let ticket_path = ticket.join(format!(".manyhands/tickets/{ticket_id}/ticket.md"));
        fs::create_dir_all(ticket_path.parent().unwrap()).unwrap();
        fs::write(ticket_path, ticket_source()).unwrap();

        let observation = observe_root(&repository, directory.path());

        assert_eq!(observation.active_contexts.len(), 2);
        assert!(observation.active_contexts.iter().any(|context| {
            context.context.branch.as_deref()
                == Some("manyhands/document/01ARZ3NDEKTSV4RRFFQ69G5FAV")
                && context.validation.items.len() == 1
        }));
        assert!(observation.active_contexts.iter().any(|context| {
            context.context.branch.as_deref() == Some("manyhands/ticket/01ARZ3NDEKTSV4RRFFQ69G5FAW")
                && context.validation.items.len() == 1
        }));
    }

    #[test]
    fn invalid_configuration_suppresses_active_context_discovery() {
        let (directory, repository) = repository_on_main();
        write_config(
            directory.path(),
            "format_version = 2\nprimary_branch = \"main\"\n",
        );
        let worktree = active_worktree(
            &repository,
            directory.path(),
            "document",
            "01ARZ3NDEKTSV4RRFFQ69G5FAV",
        );
        fs::create_dir_all(worktree.join("docs")).unwrap();
        fs::write(worktree.join("docs/document.md"), document_source()).unwrap();

        let observation = observe_root(&repository, directory.path());

        assert!(observation.active_contexts.is_empty());
    }

    #[test]
    fn wrong_root_branch_remains_unverified_while_discovering_valid_active_contexts() {
        let (directory, repository) = repository_on_main();
        write_config(
            directory.path(),
            "format_version = 1\nprimary_branch = \"main\"\n",
        );
        let worktree = active_worktree(
            &repository,
            directory.path(),
            "document",
            "01ARZ3NDEKTSV4RRFFQ69G5FAV",
        );
        fs::create_dir_all(worktree.join("docs")).unwrap();
        fs::write(worktree.join("docs/document.md"), document_source()).unwrap();
        let head = repository.head().unwrap().peel_to_commit().unwrap();
        repository.branch("other", &head, false).unwrap();
        repository.set_head("refs/heads/other").unwrap();
        repository.checkout_head(None).unwrap();

        let observation = observe_root(&repository, directory.path());

        assert_eq!(observation.context.kind, DiscoveryContextKind::Unverified);
        assert_eq!(observation.active_contexts.len(), 1);
        assert_eq!(
            observation.active_contexts[0].context.branch.as_deref(),
            Some("manyhands/document/01ARZ3NDEKTSV4RRFFQ69G5FAV")
        );
    }

    #[test]
    fn reports_a_registered_deterministic_worktree_on_a_nonconforming_branch() {
        let (directory, repository) = repository_on_main();
        write_config(
            directory.path(),
            "format_version = 1\nprimary_branch = \"main\"\n",
        );
        let worktree = active_worktree(
            &repository,
            directory.path(),
            "document",
            "01ARZ3NDEKTSV4RRFFQ69G5FAV",
        );
        let linked = Repository::open(&worktree).unwrap();
        let head = linked.head().unwrap().peel_to_commit().unwrap();
        linked.branch("other", &head, false).unwrap();
        linked.set_head("refs/heads/other").unwrap();
        linked.checkout_head(None).unwrap();

        let observation = observe_root(&repository, directory.path());

        assert!(observation.active_contexts.is_empty());
        assert!(observation.problems.iter().any(|problem| matches!(
            problem,
            RootObservationProblem::Context { path, .. } if path == &worktree
        )));
    }

    #[test]
    fn rejects_an_independent_repository_at_a_deterministic_worktree_path() {
        let (directory, repository) = repository_on_main();
        write_config(
            directory.path(),
            "format_version = 1\nprimary_branch = \"main\"\n",
        );
        let worktree = directory
            .path()
            .join(".manyhands/worktrees/01ARZ3NDEKTSV4RRFFQ69G5FAV");
        fs::create_dir_all(&worktree).unwrap();
        let mut options = RepositoryInitOptions::new();
        options.initial_head("main");
        let independent = Repository::init_opts(&worktree, &options).unwrap();
        fs::write(worktree.join("fixture.txt"), "fixture\n").unwrap();
        let mut index = independent.index().unwrap();
        index.add_path(Path::new("fixture.txt")).unwrap();
        let tree = independent.find_tree(index.write_tree().unwrap()).unwrap();
        let signature = Signature::new("Test", "test@example.invalid", &Time::new(0, 0)).unwrap();
        independent
            .commit(Some("HEAD"), &signature, &signature, "Initial", &tree, &[])
            .unwrap();
        drop(tree);
        let head = independent.head().unwrap().peel_to_commit().unwrap();
        independent
            .branch(
                "manyhands/document/01ARZ3NDEKTSV4RRFFQ69G5FAV",
                &head,
                false,
            )
            .unwrap();
        independent
            .set_head("refs/heads/manyhands/document/01ARZ3NDEKTSV4RRFFQ69G5FAV")
            .unwrap();
        fs::create_dir_all(worktree.join("docs")).unwrap();
        fs::write(worktree.join("docs/document.md"), document_source()).unwrap();

        let observation = observe_root(&repository, directory.path());

        assert!(observation.active_contexts.is_empty());
        assert!(observation.problems.iter().any(|problem| matches!(
            problem,
            RootObservationProblem::Context { path, .. } if path == &worktree
        )));
    }

    #[cfg(unix)]
    #[test]
    fn rejects_a_symlinked_worktree_base_without_traversing_external_content() {
        let (directory, repository) = repository_on_main();
        write_config(
            directory.path(),
            "format_version = 1\nprimary_branch = \"main\"\n",
        );
        let outside = tempfile::tempdir().unwrap();
        fs::write(outside.path().join("managed.md"), document_source()).unwrap();
        let base = directory.path().join(".manyhands/worktrees");
        symlink(outside.path(), &base).unwrap();

        let observation = observe_root(&repository, directory.path());

        assert!(observation.active_contexts.is_empty());
        assert!(observation.sources.is_empty());
        assert!(observation.problems.iter().any(|problem| matches!(
            problem,
            RootObservationProblem::Source { path, .. } if path == &base
        )));
    }

    #[test]
    fn reports_worktree_base_entry_overflow_without_descending_into_candidates() {
        let (directory, repository) = repository_on_main();
        write_config(
            directory.path(),
            "format_version = 1\nprimary_branch = \"main\"\n",
        );
        let base = directory.path().join(".manyhands/worktrees");
        fs::create_dir_all(&base).unwrap();
        for index in 0..=MAX_MANAGED_DIRECTORY_ENTRIES {
            fs::create_dir(base.join(format!("candidate-{index}"))).unwrap();
        }

        let observation = observe_root(&repository, directory.path());

        assert!(observation.active_contexts.is_empty());
        assert!(observation.problems.iter().any(|problem| matches!(
            problem,
            RootObservationProblem::Source { path, message }
                if path == &base && message.contains("bounded canonical source traversal limit")
        )));
    }

    #[test]
    fn rejects_mismatched_and_malformed_active_context_candidates() {
        let (directory, repository) = repository_on_main();
        write_config(
            directory.path(),
            "format_version = 1\nprimary_branch = \"main\"\n",
        );
        let worktree = active_worktree(
            &repository,
            directory.path(),
            "document",
            "01ARZ3NDEKTSV4RRFFQ69G5FAZ",
        );
        let unrelated_ticket =
            worktree.join(".manyhands/tickets/01ARZ3NDEKTSV4RRFFQ69G5FAW/ticket.md");
        fs::create_dir_all(unrelated_ticket.parent().unwrap()).unwrap();
        fs::write(unrelated_ticket, ticket_source()).unwrap();
        let path_mismatch = active_worktree_at(
            &repository,
            directory.path(),
            "ticket",
            "01ARZ3NDEKTSV4RRFFQ69G5FAW",
            "01ARZ3NDEKTSV4RRFFQ69G5FAV",
        );
        fs::create_dir_all(directory.path().join(".manyhands/worktrees/not-a-ulid")).unwrap();

        let observation = observe_root(&repository, directory.path());

        assert!(observation.active_contexts.is_empty());
        assert!(observation.problems.iter().any(|problem| matches!(
            problem,
            RootObservationProblem::Context { path, .. } if path == &worktree
        )));
        assert!(observation.problems.iter().any(|problem| matches!(
            problem,
            RootObservationProblem::Context { path, .. } if path == &path_mismatch
        )));
    }

    #[test]
    fn excludes_the_root_worktree_base_from_document_traversal() {
        let (directory, repository) = repository_on_main();
        write_config(
            directory.path(),
            "format_version = 1\nprimary_branch = \"main\"\n",
        );
        let worktree_document = directory
            .path()
            .join(".manyhands/worktrees/item/docs/hidden.md");
        fs::create_dir_all(worktree_document.parent().unwrap()).unwrap();
        fs::write(&worktree_document, document_source()).unwrap();
        let visible_document = directory.path().join("docs/visible.md");
        fs::create_dir_all(visible_document.parent().unwrap()).unwrap();
        fs::write(&visible_document, document_source()).unwrap();

        let observation = observe_root(&repository, directory.path());

        assert_eq!(MAX_DOCUMENT_DIRECTORY_DEPTH, 16);
        assert_eq!(observation.sources.len(), 1);
        assert_eq!(observation.sources[0].0, Path::new("docs/visible.md"));
    }

    #[test]
    fn collects_valid_ticket_and_comment_sources() {
        let (directory, repository) = repository_on_main();
        write_config(
            directory.path(),
            "format_version = 1\nprimary_branch = \"main\"\n",
        );
        fs::create_dir_all(directory.path().join("docs")).unwrap();
        fs::write(directory.path().join("docs/document.md"), document_source()).unwrap();
        let ticket = directory
            .path()
            .join(".manyhands/tickets/01ARZ3NDEKTSV4RRFFQ69G5FAW/ticket.md");
        fs::create_dir_all(ticket.parent().unwrap()).unwrap();
        fs::write(ticket, ticket_source()).unwrap();
        let comment = directory
            .path()
            .join(".manyhands/comments/01ARZ3NDEKTSV4RRFFQ69G5FAV/01ARZ3NDEKTSV4RRFFQ69G5FAX.md");
        fs::create_dir_all(comment.parent().unwrap()).unwrap();
        fs::write(comment, comment_source()).unwrap();

        let observation = observe_root(&repository, directory.path());

        assert_eq!(observation.sources.len(), 3);
        assert_eq!(observation.validation.items.len(), 3);
        assert!(observation.validation.problems.is_empty());
    }

    #[test]
    fn excludes_git_metadata_directories_under_docs() {
        let (directory, repository) = repository_on_main();
        write_config(
            directory.path(),
            "format_version = 1\nprimary_branch = \"main\"\n",
        );
        let metadata_document = directory.path().join("docs/.git/hidden.md");
        fs::create_dir_all(metadata_document.parent().unwrap()).unwrap();
        fs::write(metadata_document, document_source()).unwrap();
        fs::write(directory.path().join("docs/visible.md"), document_source()).unwrap();

        let observation = observe_root(&repository, directory.path());

        assert_eq!(observation.sources.len(), 1);
        assert_eq!(observation.sources[0].0, Path::new("docs/visible.md"));
    }

    #[cfg(unix)]
    #[test]
    fn reports_unsafe_or_invalid_canonical_entries_without_traversing_them() {
        let (directory, repository) = repository_on_main();
        write_config(
            directory.path(),
            "format_version = 1\nprimary_branch = \"main\"\n",
        );
        fs::create_dir_all(directory.path().join("docs")).unwrap();
        let outside = tempfile::tempdir().unwrap();
        fs::write(outside.path().join("outside.md"), document_source()).unwrap();
        let unsafe_source = directory.path().join("docs/unsafe.md");
        symlink(outside.path().join("outside.md"), &unsafe_source).unwrap();
        let invalid_comments = directory.path().join(".manyhands/comments");
        fs::create_dir_all(invalid_comments.parent().unwrap()).unwrap();
        fs::write(&invalid_comments, "not a directory").unwrap();

        let observation = observe_root(&repository, directory.path());

        assert!(observation.sources.is_empty());
        assert!(observation.problems.iter().any(|problem| matches!(
            problem,
            RootObservationProblem::Source { path, .. } if path == &unsafe_source
        )));
        assert!(observation.problems.iter().any(|problem| matches!(
            problem,
            RootObservationProblem::Source { path, .. } if path == &invalid_comments
        )));
        assert_eq!(
            fs::read_to_string(outside.path().join("outside.md")).unwrap(),
            document_source()
        );
    }

    #[cfg(unix)]
    #[test]
    fn symlinked_configuration_is_invalid_and_unverified() {
        let (directory, repository) = repository_on_main();
        let outside = tempfile::tempdir().unwrap();
        let outside_config = outside.path().join("config.toml");
        fs::write(
            &outside_config,
            "format_version = 1\nprimary_branch = \"main\"\n",
        )
        .unwrap();
        let config = directory.path().join(canonical::CONFIG_PATH);
        fs::create_dir_all(config.parent().unwrap()).unwrap();
        symlink(&outside_config, &config).unwrap();

        let observation = observe_root(&repository, directory.path());

        assert!(matches!(
            observation.configuration,
            RootConfiguration::Invalid(_)
        ));
        assert_eq!(observation.context.kind, DiscoveryContextKind::Unverified);
        assert!(
            observation
                .problems
                .iter()
                .any(|problem| matches!(problem, RootObservationProblem::Configuration(_)))
        );
    }

    #[cfg(unix)]
    #[test]
    fn reports_a_symlinked_ticket_source_without_following_it() {
        let (directory, repository) = repository_on_main();
        write_config(
            directory.path(),
            "format_version = 1\nprimary_branch = \"main\"\n",
        );
        let outside = tempfile::tempdir().unwrap();
        let outside_ticket = outside.path().join("ticket.md");
        fs::write(&outside_ticket, ticket_source()).unwrap();
        let ticket = directory
            .path()
            .join(".manyhands/tickets/01ARZ3NDEKTSV4RRFFQ69G5FAW/ticket.md");
        fs::create_dir_all(ticket.parent().unwrap()).unwrap();
        symlink(&outside_ticket, &ticket).unwrap();

        let observation = observe_root(&repository, directory.path());

        assert!(observation.sources.is_empty());
        assert!(observation.problems.iter().any(|problem| matches!(
            problem,
            RootObservationProblem::Source { path, .. } if path == &ticket
        )));
        assert_eq!(fs::read_to_string(outside_ticket).unwrap(), ticket_source());
    }

    #[test]
    fn reports_ticket_directory_entry_limit_overflow() {
        let (directory, repository) = repository_on_main();
        write_config(
            directory.path(),
            "format_version = 1\nprimary_branch = \"main\"\n",
        );
        let tickets = directory.path().join(".manyhands/tickets");
        fs::create_dir_all(&tickets).unwrap();
        for index in 0..=MAX_MANAGED_DIRECTORY_ENTRIES {
            fs::create_dir(tickets.join(format!("entry-{index}"))).unwrap();
        }

        let observation = observe_root(&repository, directory.path());

        assert!(observation.problems.iter().any(|problem| matches!(
            problem,
            RootObservationProblem::Source { path, message }
                if path == &tickets && message.contains("bounded canonical source traversal limit")
        )));
    }

    #[test]
    fn reports_comment_directory_entry_limit_overflow() {
        let (directory, repository) = repository_on_main();
        write_config(
            directory.path(),
            "format_version = 1\nprimary_branch = \"main\"\n",
        );
        let comments = directory.path().join(".manyhands/comments");
        fs::create_dir_all(&comments).unwrap();
        for index in 0..=MAX_MANAGED_DIRECTORY_ENTRIES {
            fs::create_dir(comments.join(format!("entry-{index}"))).unwrap();
        }

        let observation = observe_root(&repository, directory.path());

        assert!(observation.problems.iter().any(|problem| matches!(
            problem,
            RootObservationProblem::Source { path, message }
                if path == &comments && message.contains("bounded canonical source traversal limit")
        )));
    }
}
