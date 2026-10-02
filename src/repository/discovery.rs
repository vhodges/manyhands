#![allow(dead_code)] // The private observation slice is consumed by the later persistence task.

use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
};

use git2::Repository;
use rusqlite::{Connection, OpenFlags};
use time::OffsetDateTime;

use crate::canonical;

use super::{
    AuthoringKind, DiscoveryContextKind, MAX_DOCUMENT_DIRECTORY_DEPTH,
    MAX_DOCUMENT_DIRECTORY_ENTRIES, MAX_MANAGED_DIRECTORY_ENTRIES, REGISTRY_BUSY_TIMEOUT,
    RegistryConnectionPhase, RepositoryError,
};

const MAX_ACTIVITY_HISTORY_COMMITS: usize = 1024;

#[derive(Debug)]
pub(super) enum RootConfiguration {
    Missing,
    Valid(canonical::RepositoryConfig),
    Invalid(canonical::ValidationProblem),
}

#[derive(Debug)]
pub(super) struct RootHeadObservation {
    pub(super) branch: Option<String>,
    pub(super) oid: Option<git2::Oid>,
}

#[derive(Debug)]
pub(super) struct RootContextObservation {
    pub(super) kind: DiscoveryContextKind,
    pub(super) branch: Option<String>,
    pub(super) worktree: PathBuf,
    pub(super) item_id: Option<canonical::ItemId>,
}

#[derive(Debug)]
pub(super) enum RootObservationProblem {
    Configuration(canonical::ValidationProblem),
    Branch { message: String },
    Source { path: PathBuf, message: String },
    Context { path: PathBuf, message: String },
}

#[derive(Debug)]
pub(super) struct RootObservation {
    pub(super) configuration: RootConfiguration,
    pub(super) configuration_source: Option<Vec<u8>>,
    pub(super) configuration_blob_oid: Option<git2::Oid>,
    pub(super) context: RootContextObservation,
    pub(super) head: RootHeadObservation,
    pub(super) sources: Vec<(PathBuf, String)>,
    pub(super) validation: canonical::ValidatedContext,
    pub(super) items: Vec<ObservedItem>,
    pub(super) problems: Vec<RootObservationProblem>,
    pub(super) active_contexts: Vec<ActiveContextObservation>,
}

#[derive(Debug)]
pub(super) struct ActiveContextObservation {
    pub(super) context: RootContextObservation,
    pub(super) head: RootHeadObservation,
    pub(super) sources: Vec<(PathBuf, String)>,
    pub(super) validation: canonical::ValidatedContext,
    pub(super) items: Vec<ObservedItem>,
    pub(super) problems: Vec<RootObservationProblem>,
}

#[derive(Debug)]
pub(super) struct ObservedItem {
    pub(super) id: canonical::ItemId,
    pub(super) kind: AuthoringKind,
    pub(super) path: PathBuf,
    pub(super) title: String,
    pub(super) ticket_type: Option<String>,
    pub(super) status: Option<String>,
    pub(super) project: Option<String>,
    pub(super) team: Option<String>,
    pub(super) closed_at: Option<OffsetDateTime>,
    pub(super) comments: Vec<ObservedCommentThread>,
    pub(super) activity_at: OffsetDateTime,
    pub(super) activity_source: super::DiscoveryActivitySource,
}

#[derive(Debug)]
pub(super) struct ObservedCommentThread {
    pub(super) id: canonical::ItemId,
    pub(super) path: PathBuf,
    pub(super) created_at: OffsetDateTime,
    pub(super) replies: Vec<ObservedCommentThread>,
}

pub(super) fn observe_root(repository: &Repository, root: &Path) -> RootObservation {
    let mut problems = Vec::new();
    let configuration = observe_configuration(root, &mut problems);
    let configuration_source = fs::read(root.join(canonical::CONFIG_PATH)).ok();
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
    let items = observe_items(repository, root, &sources, &validation);
    let active_contexts = match configuration {
        RootConfiguration::Valid(_) => observe_active_contexts(repository, root, &mut problems),
        RootConfiguration::Missing | RootConfiguration::Invalid(_) => Vec::new(),
    };

    RootObservation {
        configuration_blob_oid: configuration_blob_oid(repository, configuration_source.as_deref()),
        configuration,
        configuration_source,
        context: RootContextObservation {
            kind,
            branch: head.branch.clone(),
            worktree: root.to_owned(),
            item_id: None,
        },
        head,
        sources,
        validation,
        items,
        problems,
        active_contexts,
    }
}

fn configuration_blob_oid(
    repository: &Repository,
    live_source: Option<&[u8]>,
) -> Option<git2::Oid> {
    let entry = repository
        .head()
        .ok()?
        .peel_to_tree()
        .ok()?
        .get_path(Path::new(canonical::CONFIG_PATH))
        .ok()
        .filter(|entry| entry.filemode() == 0o100644)?;
    let blob = repository.find_blob(entry.id()).ok()?;
    (live_source == Some(blob.content())).then_some(entry.id())
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
        let items = observe_items(&repository, &expected_path, &sources, &validation);
        contexts.push(ActiveContextObservation {
            context: RootContextObservation {
                kind: DiscoveryContextKind::Active,
                branch: head.branch.clone(),
                worktree: expected_path,
                item_id: Some(path_id),
            },
            head,
            sources,
            validation,
            items,
            problems: context_problems,
        });
    }
    contexts.sort_by(|left, right| left.context.branch.cmp(&right.context.branch));
    contexts
}

fn observe_items(
    repository: &Repository,
    root: &Path,
    sources: &[(PathBuf, String)],
    validation: &canonical::ValidatedContext,
) -> Vec<ObservedItem> {
    let valid_ids = validation
        .items
        .iter()
        .map(canonical_item_id)
        .collect::<BTreeSet<_>>();
    let paths = sources
        .iter()
        .filter_map(|(path, source)| {
            canonical::parse_item(path, source)
                .ok()
                .map(|item| (canonical_item_id(&item).clone(), path.clone()))
        })
        .filter(|(id, _)| valid_ids.contains(id))
        .collect::<BTreeMap<_, _>>();

    validation
        .items
        .iter()
        .filter_map(|item| match item {
            canonical::CanonicalItem::Document(document) => Some(&document.id),
            canonical::CanonicalItem::Ticket(ticket) => Some(&ticket.id),
            canonical::CanonicalItem::Comment(_) => None,
        })
        .filter_map(|id| {
            let path = paths.get(id)?.clone();
            let metadata = validation.items.iter().find_map(|item| match item {
                canonical::CanonicalItem::Document(document) if document.id == *id => Some((
                    AuthoringKind::Document,
                    document.title.clone(),
                    None,
                    None,
                    None,
                    None,
                    None,
                )),
                canonical::CanonicalItem::Ticket(ticket) if ticket.id == *id => Some((
                    AuthoringKind::Ticket,
                    ticket.title.clone(),
                    Some(ticket.ticket_type.clone()),
                    Some(ticket.status.clone()),
                    ticket.project.clone(),
                    ticket.team.clone(),
                    ticket.closed_at,
                )),
                _ => None,
            })?;
            let item_context = canonical::ValidatedContext {
                items: validation
                    .items
                    .iter()
                    .filter(|item| match item {
                        canonical::CanonicalItem::Document(document) => document.id == *id,
                        canonical::CanonicalItem::Ticket(ticket) => ticket.id == *id,
                        canonical::CanonicalItem::Comment(comment) => comment.item_id == *id,
                    })
                    .cloned()
                    .collect(),
                problems: Vec::new(),
            };
            let comments = canonical::ordered_comment_threads(&item_context)
                .into_iter()
                .filter_map(|thread| observe_comment_thread(thread, &paths))
                .collect::<Vec<_>>();
            let mut owned_paths = vec![path.clone()];
            collect_comment_paths(&comments, &mut owned_paths);
            let (activity_at, activity_source) = observe_activity(repository, root, &owned_paths)?;
            Some(ObservedItem {
                id: id.clone(),
                kind: metadata.0,
                path,
                title: metadata.1,
                ticket_type: metadata.2,
                status: metadata.3,
                project: metadata.4,
                team: metadata.5,
                closed_at: metadata.6,
                comments,
                activity_at,
                activity_source,
            })
        })
        .collect()
}

fn canonical_item_id(item: &canonical::CanonicalItem) -> &canonical::ItemId {
    match item {
        canonical::CanonicalItem::Document(document) => &document.id,
        canonical::CanonicalItem::Ticket(ticket) => &ticket.id,
        canonical::CanonicalItem::Comment(comment) => &comment.id,
    }
}

fn observe_comment_thread(
    thread: canonical::CommentThread,
    paths: &BTreeMap<canonical::ItemId, PathBuf>,
) -> Option<ObservedCommentThread> {
    let path = paths.get(&thread.comment.id)?.clone();
    Some(ObservedCommentThread {
        id: thread.comment.id,
        path,
        created_at: thread.comment.created_at,
        replies: thread
            .replies
            .into_iter()
            .filter_map(|reply| observe_comment_thread(reply, paths))
            .collect(),
    })
}

fn collect_comment_paths(comments: &[ObservedCommentThread], paths: &mut Vec<PathBuf>) {
    for comment in comments {
        paths.push(comment.path.clone());
        collect_comment_paths(&comment.replies, paths);
    }
}

fn observe_activity(
    repository: &Repository,
    root: &Path,
    paths: &[PathBuf],
) -> Option<(OffsetDateTime, super::DiscoveryActivitySource)> {
    let commit = latest_commit_touching(repository, paths);
    let modified = paths
        .iter()
        .filter_map(|path| {
            (repository.status_file(path).ok()? != git2::Status::CURRENT)
                .then(|| fs::metadata(root.join(path)).ok()?.modified().ok())
                .flatten()
        })
        .map(OffsetDateTime::from)
        .max();
    match (commit, modified) {
        (Some(commit), Some(modified)) if modified > commit => Some((
            modified,
            super::DiscoveryActivitySource::UncommittedFilesystem,
        )),
        (Some(commit), _) => Some((commit, super::DiscoveryActivitySource::GitCommit)),
        (None, Some(modified)) => Some((
            modified,
            super::DiscoveryActivitySource::UncommittedFilesystem,
        )),
        (None, None) => None,
    }
}

fn latest_commit_touching(repository: &Repository, paths: &[PathBuf]) -> Option<OffsetDateTime> {
    first_parent_commits(repository, MAX_ACTIVITY_HISTORY_COMMITS)
        .into_iter()
        .find(|commit| commit_touches_paths(repository, commit, paths))
        .and_then(|commit| OffsetDateTime::from_unix_timestamp(commit.time().seconds()).ok())
}

fn first_parent_commits<'repository>(
    repository: &'repository Repository,
    maximum: usize,
) -> Vec<git2::Commit<'repository>> {
    let mut next = repository
        .head()
        .ok()
        .and_then(|head| head.peel_to_commit().ok());
    let mut commits = Vec::new();
    while commits.len() < maximum {
        let Some(commit) = next else {
            break;
        };
        next = commit.parent(0).ok();
        commits.push(commit);
    }
    commits
}

fn commit_touches_paths(
    repository: &Repository,
    commit: &git2::Commit<'_>,
    paths: &[PathBuf],
) -> bool {
    let Ok(tree) = commit.tree() else {
        return false;
    };
    let parent = commit.parent(0).ok().and_then(|parent| parent.tree().ok());
    repository
        .diff_tree_to_tree(parent.as_ref(), Some(&tree), None)
        .ok()
        .is_some_and(|diff| {
            diff.deltas().any(|delta| {
                [delta.old_file().path(), delta.new_file().path()]
                    .into_iter()
                    .flatten()
                    .any(|path| paths.iter().any(|owned| owned == path))
            })
        })
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
                 state TEXT NOT NULL DEFAULT 'completed',
                 context_path TEXT,
                 persisted_context_count INTEGER NOT NULL DEFAULT 0,
                 observed_at INTEGER NOT NULL
              );
             CREATE TABLE IF NOT EXISTS index_operation_contexts (
                 operation_id INTEGER NOT NULL REFERENCES index_operations(id) ON DELETE CASCADE,
                 worktree_path TEXT NOT NULL,
                 observation_fingerprint TEXT NOT NULL DEFAULT '',
                 PRIMARY KEY (operation_id, worktree_path)
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
    let has_state = transaction
        .query_row(
            "SELECT COUNT(*) FROM pragma_table_info('index_operations') WHERE name = 'state'",
            [],
            |row| row.get::<_, i64>(0),
        )
        .map_err(RepositoryError::sqlite)?
        != 0;
    if !has_state {
        transaction
            .execute_batch(
                "ALTER TABLE index_operations ADD COLUMN state TEXT NOT NULL DEFAULT 'completed';
             ALTER TABLE index_operations ADD COLUMN context_path TEXT;",
            )
            .map_err(RepositoryError::sqlite)?;
    }
    let has_persisted_context_count = transaction.query_row(
        "SELECT COUNT(*) FROM pragma_table_info('index_operations') WHERE name = 'persisted_context_count'",
        [], |row| row.get::<_, i64>(0),
    ).map_err(RepositoryError::sqlite)? != 0;
    if !has_persisted_context_count {
        transaction.execute_batch("ALTER TABLE index_operations ADD COLUMN persisted_context_count INTEGER NOT NULL DEFAULT 0;")
            .map_err(RepositoryError::sqlite)?;
    }
    transaction.commit().map_err(RepositoryError::sqlite)
}

#[cfg(test)]
mod tests {
    #[cfg(unix)]
    use std::os::unix::fs::symlink;
    use std::{fs, path::Path};

    use git2::{Repository, RepositoryInitOptions, Signature, Time, WorktreeAddOptions};
    use time::OffsetDateTime;

    use super::*;
    use crate::{
        canonical,
        repository::{DiscoveryActivitySource, DiscoveryContextKind, MAX_DOCUMENT_DIRECTORY_DEPTH},
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

    fn comment_source_with(id: &str, parent_id: Option<&str>, created_at: &str) -> String {
        let parent_id = parent_id
            .map(|parent_id| format!("parent_id: \"{parent_id}\"\n"))
            .unwrap_or_default();
        format!(
            "---\nmanyhands_managed: true\nmanyhands_kind: comment\nid: \"{id}\"\nitem_id: \"01ARZ3NDEKTSV4RRFFQ69G5FAV\"\n{parent_id}created_at: {created_at}\n---\n"
        )
    }

    fn checkpoint(repository: &Repository, path: &Path, timestamp: i64) -> git2::Oid {
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

    fn checkpoint_reference(
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

    fn observed_document(observation: &RootObservation) -> &ObservedItem {
        observation
            .items
            .iter()
            .find(|item| item.id.to_string() == "01ARZ3NDEKTSV4RRFFQ69G5FAV")
            .unwrap()
    }

    #[test]
    fn observation_orders_equal_time_comment_siblings_by_ulid_recursively() {
        let (directory, repository) = repository_on_main();
        write_config(
            directory.path(),
            "format_version = 1\nprimary_branch = \"main\"\n",
        );
        fs::create_dir_all(directory.path().join("docs")).unwrap();
        fs::write(directory.path().join("docs/document.md"), document_source()).unwrap();
        let comments = directory
            .path()
            .join(".manyhands/comments/01ARZ3NDEKTSV4RRFFQ69G5FAV");
        fs::create_dir_all(&comments).unwrap();
        fs::write(
            comments.join("01C00000000000000000000001.md"),
            comment_source_with("01C00000000000000000000001", None, "2026-09-30T12:00:00Z"),
        )
        .unwrap();
        fs::write(
            comments.join("01B00000000000000000000001.md"),
            comment_source_with("01B00000000000000000000001", None, "2026-09-30T12:00:00Z"),
        )
        .unwrap();
        fs::write(
            comments.join("01E00000000000000000000001.md"),
            comment_source_with(
                "01E00000000000000000000001",
                Some("01B00000000000000000000001"),
                "2026-09-30T12:00:00Z",
            ),
        )
        .unwrap();
        fs::write(
            comments.join("01D00000000000000000000001.md"),
            comment_source_with(
                "01D00000000000000000000001",
                Some("01B00000000000000000000001"),
                "2026-09-30T12:00:00Z",
            ),
        )
        .unwrap();

        let observation = observe_root(&repository, directory.path());
        let item = observed_document(&observation);

        assert_eq!(
            item.comments[0].id.to_string(),
            "01B00000000000000000000001"
        );
        assert_eq!(
            item.comments[1].id.to_string(),
            "01C00000000000000000000001"
        );
        assert_eq!(
            item.comments[0].replies[0].id.to_string(),
            "01D00000000000000000000001"
        );
        assert_eq!(
            item.comments[0].replies[1].id.to_string(),
            "01E00000000000000000000001"
        );
    }

    #[test]
    fn observation_uses_a_newer_comment_commit_for_item_activity() {
        let (directory, repository) = repository_on_main();
        write_config(
            directory.path(),
            "format_version = 1\nprimary_branch = \"main\"\n",
        );
        let document = directory.path().join("docs/document.md");
        fs::create_dir_all(document.parent().unwrap()).unwrap();
        fs::write(&document, document_source()).unwrap();
        checkpoint(&repository, Path::new("docs/document.md"), 10);
        let comment = directory
            .path()
            .join(".manyhands/comments/01ARZ3NDEKTSV4RRFFQ69G5FAV/01ARZ3NDEKTSV4RRFFQ69G5FAX.md");
        fs::create_dir_all(comment.parent().unwrap()).unwrap();
        fs::write(&comment, comment_source()).unwrap();
        checkpoint(
            &repository,
            Path::new(
                ".manyhands/comments/01ARZ3NDEKTSV4RRFFQ69G5FAV/01ARZ3NDEKTSV4RRFFQ69G5FAX.md",
            ),
            20,
        );

        let observation = observe_root(&repository, directory.path());
        let item = observed_document(&observation);

        assert_eq!(
            item.activity_at,
            OffsetDateTime::from_unix_timestamp(20).unwrap()
        );
        assert_eq!(item.activity_source, DiscoveryActivitySource::GitCommit);
    }

    #[test]
    fn observation_uses_the_newer_first_parent_commit_when_its_timestamp_is_backdated() {
        let (directory, repository) = repository_on_main();
        write_config(
            directory.path(),
            "format_version = 1\nprimary_branch = \"main\"\n",
        );
        let document = directory.path().join("docs/document.md");
        fs::create_dir_all(document.parent().unwrap()).unwrap();
        fs::write(&document, document_source()).unwrap();
        checkpoint(&repository, Path::new("docs/document.md"), 100);
        fs::write(&document, format!("{}newer revision\n", document_source())).unwrap();
        checkpoint(&repository, Path::new("docs/document.md"), 10);

        let observation = observe_root(&repository, directory.path());
        let item = observed_document(&observation);

        assert_eq!(
            item.activity_at,
            OffsetDateTime::from_unix_timestamp(10).unwrap()
        );
        assert_eq!(item.activity_source, DiscoveryActivitySource::GitCommit);
    }

    #[test]
    fn observation_ignores_a_newer_side_branch_only_item_change() {
        let (directory, repository) = repository_on_main();
        write_config(
            directory.path(),
            "format_version = 1\nprimary_branch = \"main\"\n",
        );
        let document = directory.path().join("docs/document.md");
        fs::create_dir_all(document.parent().unwrap()).unwrap();
        fs::write(&document, document_source()).unwrap();
        let base = checkpoint(&repository, Path::new("docs/document.md"), 10);
        repository
            .branch("side", &repository.find_commit(base).unwrap(), false)
            .unwrap();
        fs::write(&document, format!("{}side branch\n", document_source())).unwrap();
        let side = checkpoint_reference(
            &repository,
            Path::new("docs/document.md"),
            "refs/heads/side",
            base,
            100,
        );
        let base_object = repository.find_object(base, None).unwrap();
        repository
            .reset(&base_object, git2::ResetType::Hard, None)
            .unwrap();
        let main = checkpoint(&repository, Path::new("fixture.txt"), 20);
        let main_commit = repository.find_commit(main).unwrap();
        let side_commit = repository.find_commit(side).unwrap();
        let tree = main_commit.tree().unwrap();
        let signature = Signature::new("Test", "test@example.invalid", &Time::new(30, 0)).unwrap();
        repository
            .commit(
                Some("HEAD"),
                &signature,
                &signature,
                "Merge side without its item change",
                &tree,
                &[&main_commit, &side_commit],
            )
            .unwrap();

        let observation = observe_root(&repository, directory.path());
        let item = observed_document(&observation);

        assert_eq!(
            item.activity_at,
            OffsetDateTime::from_unix_timestamp(10).unwrap()
        );
    }

    #[test]
    fn first_parent_history_respects_the_requested_inspection_bound() {
        let (_directory, repository) = repository_on_main();
        checkpoint(&repository, Path::new("fixture.txt"), 1);
        checkpoint(&repository, Path::new("fixture.txt"), 2);
        checkpoint(&repository, Path::new("fixture.txt"), 3);

        let commits = first_parent_commits(&repository, 2);

        assert_eq!(MAX_ACTIVITY_HISTORY_COMMITS, 1024);
        assert_eq!(commits.len(), 2);
        assert_eq!(commits[0].time().seconds(), 3);
        assert_eq!(commits[1].time().seconds(), 2);
    }

    #[test]
    fn observation_uses_newer_uncommitted_item_or_comment_mtime() {
        for path in [
            PathBuf::from("docs/document.md"),
            PathBuf::from(
                ".manyhands/comments/01ARZ3NDEKTSV4RRFFQ69G5FAV/01ARZ3NDEKTSV4RRFFQ69G5FAX.md",
            ),
        ] {
            let (directory, repository) = repository_on_main();
            write_config(
                directory.path(),
                "format_version = 1\nprimary_branch = \"main\"\n",
            );
            let document = directory.path().join("docs/document.md");
            fs::create_dir_all(document.parent().unwrap()).unwrap();
            fs::write(&document, document_source()).unwrap();
            let comment = directory.path().join(
                ".manyhands/comments/01ARZ3NDEKTSV4RRFFQ69G5FAV/01ARZ3NDEKTSV4RRFFQ69G5FAX.md",
            );
            fs::create_dir_all(comment.parent().unwrap()).unwrap();
            fs::write(&comment, comment_source()).unwrap();
            checkpoint(&repository, Path::new("docs/document.md"), 10);
            checkpoint(
                &repository,
                Path::new(
                    ".manyhands/comments/01ARZ3NDEKTSV4RRFFQ69G5FAV/01ARZ3NDEKTSV4RRFFQ69G5FAX.md",
                ),
                20,
            );
            let source = if path == Path::new("docs/document.md") {
                document_source().to_owned()
            } else {
                comment_source().to_owned()
            };
            fs::write(
                directory.path().join(&path),
                format!("{source}uncommitted\n"),
            )
            .unwrap();

            let observation = observe_root(&repository, directory.path());
            let item = observed_document(&observation);

            assert_eq!(
                item.activity_source,
                DiscoveryActivitySource::UncommittedFilesystem
            );
            assert!(item.activity_at > OffsetDateTime::from_unix_timestamp(20).unwrap());
        }
    }

    #[test]
    fn observation_excludes_invalid_comments_from_trees_and_activity() {
        let (directory, repository) = repository_on_main();
        write_config(
            directory.path(),
            "format_version = 1\nprimary_branch = \"main\"\n",
        );
        let document = directory.path().join("docs/document.md");
        fs::create_dir_all(document.parent().unwrap()).unwrap();
        fs::write(&document, document_source()).unwrap();
        checkpoint(&repository, Path::new("docs/document.md"), 10);
        let invalid = directory
            .path()
            .join(".manyhands/comments/01ARZ3NDEKTSV4RRFFQ69G5FAV/01ARZ3NDEKTSV4RRFFQ69G5FAX.md");
        fs::create_dir_all(invalid.parent().unwrap()).unwrap();
        fs::write(
            &invalid,
            comment_source_with(
                "01ARZ3NDEKTSV4RRFFQ69G5FAX",
                Some("01ARZ3NDEKTSV4RRFFQ69G5FAY"),
                "2026-09-30T12:00:00Z",
            ),
        )
        .unwrap();
        checkpoint(
            &repository,
            Path::new(
                ".manyhands/comments/01ARZ3NDEKTSV4RRFFQ69G5FAV/01ARZ3NDEKTSV4RRFFQ69G5FAX.md",
            ),
            20,
        );

        let observation = observe_root(&repository, directory.path());
        let item = observed_document(&observation);

        assert!(item.comments.is_empty());
        assert_eq!(
            item.activity_at,
            OffsetDateTime::from_unix_timestamp(10).unwrap()
        );
    }

    #[test]
    fn observation_does_not_mutate_git_state() {
        let (directory, repository) = repository_on_main();
        write_config(
            directory.path(),
            "format_version = 1\nprimary_branch = \"main\"\n",
        );
        fs::create_dir_all(directory.path().join("docs")).unwrap();
        fs::write(directory.path().join("docs/document.md"), document_source()).unwrap();
        let head = repository.head().unwrap().target();
        let index = fs::read(repository.path().join("index")).unwrap();
        let config = fs::read(repository.path().join("config")).unwrap();

        let _ = observe_root(&repository, directory.path());

        assert_eq!(repository.head().unwrap().target(), head);
        assert_eq!(fs::read(repository.path().join("index")).unwrap(), index);
        assert_eq!(fs::read(repository.path().join("config")).unwrap(), config);
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
