//! Deliberate synchronization composes the owned state and scoped transport seams.
#![allow(clippy::result_large_err)]
#[cfg(test)]
#[path = "sync_tests.rs"]
mod tests;
#[cfg(windows)]
#[path = "windows_resolution.rs"]
mod windows_resolution;
use super::*;
use crate::repository::{keys::*, transport::*, *};
use state::{
    SynchronizationAuthority as Authority, SynchronizationCheckpoint as Checkpoint,
    SynchronizationEvidence as Evidence,
};
#[cfg(windows)]
use windows_resolution::{
    ResolutionIndexLock, open_ref_log_file, ref_log_stamp, ref_root_identity,
    refuse_ambiguous_resolution_backend_locks, sync_git_role,
};
#[cfg(test)]
type ResolutionIndexLockHook = std::sync::Mutex<Vec<(PathBuf, Box<dyn FnOnce() + Send>)>>;
#[cfg(test)]
static RESOLUTION_INDEX_LOCK_HOOK: std::sync::OnceLock<ResolutionIndexLockHook> =
    std::sync::OnceLock::new();
#[cfg(test)]
static RESOLUTION_INDEX_PERSIST_HOOK: std::sync::OnceLock<ResolutionIndexLockHook> =
    std::sync::OnceLock::new();
#[cfg(test)]
static RESOLUTION_INDEX_SCRATCH_HOOK: std::sync::OnceLock<ResolutionIndexLockHook> =
    std::sync::OnceLock::new();
#[cfg(test)]
static RESOLUTION_INDEX_INSTALL_HOOK: std::sync::OnceLock<ResolutionIndexLockHook> =
    std::sync::OnceLock::new();
#[cfg(test)]
static RESOLUTION_INDEX_EFFECT_HOOK: std::sync::OnceLock<ResolutionIndexLockHook> =
    std::sync::OnceLock::new();
#[cfg(test)]
static RESOLUTION_INDEX_RETIRE_HOOK: std::sync::OnceLock<ResolutionIndexLockHook> =
    std::sync::OnceLock::new();
#[cfg(test)]
static RESOLUTION_REF_REFRESH_HOOK: std::sync::OnceLock<ResolutionIndexLockHook> =
    std::sync::OnceLock::new();
#[cfg(test)]
static LOCAL_RECONCILIATION_PREPARED_HOOK: std::sync::OnceLock<ResolutionIndexLockHook> =
    std::sync::OnceLock::new();

#[cfg(test)]
fn resolution_hook_root(root: &Path) -> PathBuf {
    // Fixture aliases and libgit2's plain Windows paths must key the same
    // test callback as canonical public roots. Missing roots retain their key;
    // this best-effort lookup is never used for production authorization.
    root.canonicalize().unwrap_or_else(|_| root.to_owned())
}

#[cfg(test)]
fn set_resolution_index_lock_hook(root: PathBuf, hook: impl FnOnce() + Send + 'static) {
    RESOLUTION_INDEX_LOCK_HOOK
        .get_or_init(|| std::sync::Mutex::new(Vec::new()))
        .lock()
        .expect("resolution index-lock hook")
        .push((resolution_hook_root(&root), Box::new(hook)));
}

#[cfg(test)]
fn set_resolution_index_persist_hook(root: PathBuf, hook: impl FnOnce() + Send + 'static) {
    RESOLUTION_INDEX_PERSIST_HOOK
        .get_or_init(|| std::sync::Mutex::new(Vec::new()))
        .lock()
        .expect("resolution index-persist hook")
        .push((resolution_hook_root(&root), Box::new(hook)));
}

#[cfg(test)]
fn set_resolution_index_scratch_hook(root: PathBuf, hook: impl FnOnce() + Send + 'static) {
    RESOLUTION_INDEX_SCRATCH_HOOK
        .get_or_init(|| std::sync::Mutex::new(Vec::new()))
        .lock()
        .expect("resolution index-scratch hook")
        .push((resolution_hook_root(&root), Box::new(hook)));
}

#[cfg(test)]
fn set_resolution_index_install_hook(root: PathBuf, hook: impl FnOnce() + Send + 'static) {
    RESOLUTION_INDEX_INSTALL_HOOK
        .get_or_init(|| std::sync::Mutex::new(Vec::new()))
        .lock()
        .expect("resolution index-install hook")
        .push((resolution_hook_root(&root), Box::new(hook)));
}

#[cfg(test)]
fn set_resolution_index_effect_hook(root: PathBuf, hook: impl FnOnce() + Send + 'static) {
    RESOLUTION_INDEX_EFFECT_HOOK
        .get_or_init(|| std::sync::Mutex::new(Vec::new()))
        .lock()
        .expect("resolution index-effect hook")
        .push((resolution_hook_root(&root), Box::new(hook)));
}

#[cfg(test)]
fn set_resolution_index_retire_hook(root: PathBuf, hook: impl FnOnce() + Send + 'static) {
    RESOLUTION_INDEX_RETIRE_HOOK
        .get_or_init(|| std::sync::Mutex::new(Vec::new()))
        .lock()
        .expect("resolution index-retire hook")
        .push((resolution_hook_root(&root), Box::new(hook)));
}

#[cfg(test)]
fn set_resolution_ref_refresh_hook(root: PathBuf, hook: impl FnOnce() + Send + 'static) {
    RESOLUTION_REF_REFRESH_HOOK
        .get_or_init(|| std::sync::Mutex::new(Vec::new()))
        .lock()
        .expect("resolution ref-refresh hook")
        .push((resolution_hook_root(&root), Box::new(hook)));
}

#[cfg(test)]
fn set_local_reconciliation_prepared_hook(root: PathBuf, hook: impl FnOnce() + Send + 'static) {
    LOCAL_RECONCILIATION_PREPARED_HOOK
        .get_or_init(|| std::sync::Mutex::new(Vec::new()))
        .lock()
        .expect("local reconciliation prepared hook")
        .push((resolution_hook_root(&root), Box::new(hook)));
}

#[cfg(test)]
fn run_resolution_index_hook(hooks: &std::sync::OnceLock<ResolutionIndexLockHook>, root: &Path) {
    let root = resolution_hook_root(root);
    let hook = {
        let mut hooks = hooks
            .get_or_init(|| std::sync::Mutex::new(Vec::new()))
            .lock()
            .expect("resolution index hook");
        hooks
            .iter()
            .position(|(expected_root, _)| expected_root == &root)
            .map(|position| hooks.remove(position).1)
    };
    if let Some(hook) = hook {
        hook();
    }
}

#[cfg(test)]
fn run_resolution_index_lock_hook(root: &Path) {
    run_resolution_index_hook(&RESOLUTION_INDEX_LOCK_HOOK, root);
}

#[cfg(test)]
fn run_resolution_index_persist_hook(root: &Path) {
    run_resolution_index_hook(&RESOLUTION_INDEX_PERSIST_HOOK, root);
}

#[cfg(test)]
fn run_resolution_index_scratch_hook(root: &Path) {
    run_resolution_index_hook(&RESOLUTION_INDEX_SCRATCH_HOOK, root);
}

#[cfg(test)]
fn run_resolution_index_install_hook(root: &Path) {
    run_resolution_index_hook(&RESOLUTION_INDEX_INSTALL_HOOK, root);
}

#[cfg(test)]
fn run_resolution_index_effect_hook(root: &Path) {
    run_resolution_index_hook(&RESOLUTION_INDEX_EFFECT_HOOK, root);
}

#[cfg(test)]
fn run_resolution_index_retire_hook(root: &Path) {
    run_resolution_index_hook(&RESOLUTION_INDEX_RETIRE_HOOK, root);
}

#[cfg(test)]
use std::path::PathBuf;
use std::{fmt, io::Read, path::Path};

#[cfg(unix)]
use std::{
    ffi::CString,
    io::{Seek, SeekFrom, Write},
    os::{
        fd::{AsRawFd, FromRawFd},
        unix::{ffi::OsStrExt, fs::MetadataExt},
    },
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PublishPendingReason {
    NoPublicationRemote,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SynchronizationOutcome {
    Published {
        target: SynchronizationTarget,
        oid: git2::Oid,
    },
    AlreadyCurrent {
        target: SynchronizationTarget,
        oid: git2::Oid,
    },
    PublishPending {
        target: SynchronizationTarget,
        local_oid: git2::Oid,
        reason: PublishPendingReason,
    },
}
/// A redacted inspected conflict entry. The path is represented only by its
/// capability token; callers cannot substitute a repository path.
#[derive(Clone, Debug)]
pub struct SynchronizationConflictPath {
    pub token: merge::ConflictPathToken,
    pub eligibility: merge::ConflictEligibility,
}

/// Actual Git state correlated with one retained durable integration step.
#[derive(Clone, Debug)]
pub struct SynchronizationConflictInspection {
    pub operation_id: OperationId,
    pub target: SynchronizationTarget,
    pub stage: SynchronizationStage,
    pub local_parent: git2::Oid,
    pub incoming_parent: git2::Oid,
    pub observation: merge::ConflictObservation,
    pub paths: Vec<SynchronizationConflictPath>,
}

/// Explicit, ephemeral conflict sides. Debug formatting redacts every body.
#[derive(Clone)]
pub struct EphemeralSynchronizationConflictSides {
    pub base: Option<merge::RedactedConflictBytes>,
    pub local: Option<merge::RedactedConflictBytes>,
    pub incoming: Option<merge::RedactedConflictBytes>,
    pub current: Option<merge::RedactedConflictBytes>,
}

impl fmt::Debug for EphemeralSynchronizationConflictSides {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("EphemeralSynchronizationConflictSides(<redacted>)")
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SynchronizationStage {
    Context,
    Primary,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SynchronizationResult {
    Complete(SynchronizationOutcome),
    IndexPending(IndexPending<SynchronizationOutcome>),
}
#[derive(Debug)]
pub enum SynchronizationError {
    Repository(RepositoryError),
    Transport(SshTransportError),
    Busy,
    PollYielding,
    Interrupted,
    TargetNotMaterialized,
    WorktreeNotClean {
        target: SynchronizationTarget,
    },
    WorktreeConflicted {
        target: SynchronizationTarget,
    },
    PrimaryMissing,
    RemoteContextDeleted,
    HistoryUnknown,
    /// A real Git merge is installed in the target worktree. The branch HEAD
    /// remains on the local parent and callers must inspect/repair explicitly.
    ConflictPending {
        target: SynchronizationTarget,
        operation_id: OperationId,
        stage: SynchronizationStage,
    },
    /// A divergent merge needs a committing identity; no candidate or merge
    /// state was created.
    IdentityRequired {
        target: SynchronizationTarget,
    },
    ExternalResolutionRequired {
        target: SynchronizationTarget,
        operation_id: OperationId,
    },
    MergeRequired {
        target: SynchronizationTarget,
    },
    PushRejected,
    ExternalChange,
    RecoveryRequired,
}
impl fmt::Display for SynchronizationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Repository(_) => "synchronization repository unavailable",
            Self::Transport(_) => "synchronization transport unavailable",
            Self::Busy => "remote operation busy",
            Self::PollYielding => "automatic observation yielding",
            Self::Interrupted => "synchronization interrupted",
            Self::TargetNotMaterialized => "target not materialized",
            Self::WorktreeNotClean { .. } => "target worktree not clean",
            Self::WorktreeConflicted { .. } => "target worktree conflicted",
            Self::PrimaryMissing => "primary missing",
            Self::RemoteContextDeleted => "remote context deleted",
            Self::HistoryUnknown => "publication history unknown",
            Self::ConflictPending { .. } => "synchronization conflict pending",
            Self::IdentityRequired { .. } => "commit identity required",
            Self::ExternalResolutionRequired { .. } => "external conflict resolution required",
            Self::MergeRequired { .. } => "merge required",
            Self::PushRejected => "push rejected",
            Self::ExternalChange => "external change",
            Self::RecoveryRequired => "synchronization recovery required",
        })
    }
}
impl std::error::Error for SynchronizationError {}
impl From<RepositoryError> for SynchronizationError {
    fn from(error: RepositoryError) -> Self {
        Self::Repository(error)
    }
}
impl From<SshTransportError> for SynchronizationError {
    fn from(error: SshTransportError) -> Self {
        if error.kind == SshTransportErrorKind::PushRejected {
            Self::PushRejected
        } else {
            Self::Transport(error)
        }
    }
}
#[derive(Clone, Copy)]
enum PushAbsenceBoundary {
    Deleted,
    Unknown,
    Ambiguous,
}
impl PushAbsenceBoundary {
    fn error(self) -> SynchronizationError {
        match self {
            Self::Deleted => SynchronizationError::RemoteContextDeleted,
            Self::Unknown => SynchronizationError::HistoryUnknown,
            Self::Ambiguous => SynchronizationError::RecoveryRequired,
        }
    }
}
fn unverified_push_error(error: SynchronizationError) -> SynchronizationError {
    match error {
        SynchronizationError::ExternalChange => SynchronizationError::RecoveryRequired,
        SynchronizationError::Transport(error)
            if error.kind == SshTransportErrorKind::EndpointChanged =>
        {
            SynchronizationError::RecoveryRequired
        }
        other => other,
    }
}
fn decision(value: RemoteSafePointOutcome) -> Result<(), SynchronizationError> {
    match value {
        RemoteSafePointOutcome::Continue => Ok(()),
        _ => Err(SynchronizationError::Interrupted),
    }
}

// Worktree paths are derived here and never enter requests, records or errors.
fn local_target(
    root: &Path,
    primary: &str,
    target: &SynchronizationTarget,
) -> Result<git2::Repository, SynchronizationError> {
    let linked = materialized_target(root, primary, target)?;
    if let SynchronizationTarget::Context { kind, item_id } = target {
        let repository =
            git2::Repository::open(root).map_err(|_| SynchronizationError::RecoveryRequired)?;
        let context = ItemContext {
            root: root.to_owned(),
            kind: *kind,
            item_id: item_id.clone(),
            branch: format!("manyhands/{}/{}", authoring_kind_segment(kind), item_id),
            worktree: root.join(".manyhands/worktrees").join(item_id.to_string()),
        };
        validate_context_worktree(
            &repository,
            &context,
            true,
            RepositoryOperation::RepositorySnapshot,
        )
        .map_err(|_| SynchronizationError::TargetNotMaterialized)?;
    }
    require_clean_target(&linked, target)?;
    Ok(linked)
}

/// Identity validation is also needed before inspecting an owned unfinished
/// merge. It deliberately makes no claim about cleanliness or effect proof.
fn materialized_target(
    root: &Path,
    primary: &str,
    target: &SynchronizationTarget,
) -> Result<git2::Repository, SynchronizationError> {
    let repository =
        git2::Repository::open(root).map_err(|_| SynchronizationError::RecoveryRequired)?;
    let (path, branch) = match target {
        SynchronizationTarget::Primary => (root.to_owned(), primary.to_owned()),
        SynchronizationTarget::Context { kind, item_id } => {
            let context = ItemContext {
                root: root.to_owned(),
                kind: *kind,
                item_id: item_id.clone(),
                branch: format!("manyhands/{}/{}", authoring_kind_segment(kind), item_id),
                worktree: root.join(".manyhands/worktrees").join(item_id.to_string()),
            };
            validate_existing_authoring_worktree_path(
                &context,
                RepositoryOperation::RepositorySnapshot,
            )
            .map_err(|_| SynchronizationError::TargetNotMaterialized)?;
            let registered = repository
                .find_worktree(&item_id.to_string())
                .map_err(|_| SynchronizationError::TargetNotMaterialized)?;
            if registered.path().canonicalize().ok() != context.worktree.canonicalize().ok() {
                return Err(SynchronizationError::TargetNotMaterialized);
            }
            (context.worktree, context.branch)
        }
    };
    if repository.is_worktree() {
        return Err(SynchronizationError::TargetNotMaterialized);
    }
    if matches!(target, SynchronizationTarget::Primary)
        && repository
            .find_reference(&format!("refs/heads/{primary}"))
            .is_err()
    {
        return Err(SynchronizationError::PrimaryMissing);
    }
    let linked =
        git2::Repository::open(path).map_err(|_| SynchronizationError::TargetNotMaterialized)?;
    if linked
        .find_reference("HEAD")
        .ok()
        .and_then(|r| r.symbolic_target().map(str::to_owned))
        != Some(format!("refs/heads/{branch}"))
    {
        return Err(SynchronizationError::TargetNotMaterialized);
    }
    Ok(linked)
}

fn require_clean_target(
    linked: &git2::Repository,
    target: &SynchronizationTarget,
) -> Result<(), SynchronizationError> {
    // A new synchronization must never adopt a foreign merge, rebase, or
    // cherry-pick merely because libgit2's state cache or index looks clean.
    let foreign_state = [
        "MERGE_HEAD",
        "MERGE_MSG",
        "MERGE_MODE",
        "REBASE_HEAD",
        "CHERRY_PICK_HEAD",
        "rebase-apply",
        "rebase-merge",
    ]
    .iter()
    .any(|name| {
        [linked.path(), linked.commondir()].into_iter().any(|root| {
            !matches!(
                std::fs::symlink_metadata(root.join(name)),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound
            )
        })
    });
    if foreign_state
        || linked.state() != git2::RepositoryState::Clean
        || linked
            .index()
            .map_err(|_| SynchronizationError::RecoveryRequired)?
            .has_conflicts()
    {
        return Err(SynchronizationError::WorktreeConflicted {
            target: target.clone(),
        });
    }
    let mut options = git2::StatusOptions::new();
    options.include_untracked(true).recurse_untracked_dirs(true);
    let statuses = linked
        .statuses(Some(&mut options))
        .map_err(|_| SynchronizationError::RecoveryRequired)?;
    if !statuses.is_empty() {
        return Err(SynchronizationError::WorktreeNotClean {
            target: target.clone(),
        });
    }
    drop(statuses);
    // A clean index/worktree must actually describe the checked-out commit.
    let commit = linked
        .head()
        .and_then(|h| h.peel_to_commit())
        .map_err(|_| SynchronizationError::RecoveryRequired)?;
    let tree = linked
        .index()
        .and_then(|mut i| i.write_tree())
        .map_err(|_| SynchronizationError::RecoveryRequired)?;
    if tree != commit.tree_id() {
        return Err(SynchronizationError::RecoveryRequired);
    }
    drop(commit);
    Ok(())
}
fn local_oid(repository: &git2::Repository) -> Result<git2::Oid, SynchronizationError> {
    repository
        .head()
        .and_then(|h| h.peel_to_commit())
        .map(|c| c.id())
        .map_err(|_| SynchronizationError::RecoveryRequired)
}
fn advertised_oid(
    advertised: &[(String, git2::Oid)],
    name: &str,
) -> Result<Option<git2::Oid>, SynchronizationError> {
    let mut found = advertised.iter().filter(|(n, _)| n == name);
    let oid = found.next().map(|(_, oid)| *oid);
    if found.next().is_some() {
        return Err(SynchronizationError::ExternalChange);
    }
    Ok(oid)
}
fn target_ref(plan: &RemoteRefPlan, target: &SynchronizationTarget) -> RemoteRefTarget {
    match target {
        SynchronizationTarget::Primary => plan.primary().clone(),
        SynchronizationTarget::Context { kind, item_id } => plan.context(*kind, item_id),
    }
}
struct PreparedMergeEntry {
    entry: git2::IndexEntry,
    kind: git2::ObjectType,
    bytes: Vec<u8>,
}

struct PreparedMerge {
    entries: Vec<PreparedMergeEntry>,
}

/// Ephemeral ODB-only source evidence. Shape and bytes are retained even when
/// decoding/parsing fails; neither failure can hide a changed canonical path.
struct CanonicalTreeSnapshot {
    sources: std::collections::BTreeMap<Vec<u8>, Vec<u8>>,
    entries: std::collections::BTreeMap<Vec<u8>, (git2::Oid, i32, Option<git2::ObjectType>)>,
}

enum MergePreparation {
    Clean(PreparedMerge),
    Conflict,
}

fn stage_name(stage: merge::IntegrationStage) -> SynchronizationStage {
    match stage {
        merge::IntegrationStage::Context => SynchronizationStage::Context,
        merge::IntegrationStage::Primary => SynchronizationStage::Primary,
    }
}

fn index_digest(tree: git2::Oid) -> [u8; 32] {
    *blake3::hash(tree.as_bytes()).as_bytes()
}

/// Merge preparation is deliberately done through a separately opened handle
/// with a high-priority memory ODB. The returned index has only regular stage-0
/// entries; generated blob bytes are imported and verified only while applying.
fn prepare_clean_merge(
    path: &Path,
    local: git2::Oid,
    incoming: git2::Oid,
) -> Result<MergePreparation, SynchronizationError> {
    let worker =
        git2::Repository::open(path).map_err(|_| SynchronizationError::RecoveryRequired)?;
    let odb = worker
        .odb()
        .map_err(|_| SynchronizationError::RecoveryRequired)?;
    let _mempack = odb
        .add_new_mempack_backend(1000)
        .map_err(|_| SynchronizationError::RecoveryRequired)?;
    let local = worker
        .find_commit(local)
        .map_err(|_| SynchronizationError::RecoveryRequired)?;
    let incoming = worker
        .find_commit(incoming)
        .map_err(|_| SynchronizationError::RecoveryRequired)?;
    let index = worker
        .merge_commits(&local, &incoming, None)
        .map_err(|_| SynchronizationError::RecoveryRequired)?;
    if index.has_conflicts() {
        return Ok(MergePreparation::Conflict);
    }
    let entries = index
        .iter()
        .filter(|entry| entry.flags & 0x3000 == 0)
        .map(|entry| {
            let object = odb
                .read(entry.id)
                .map_err(|_| SynchronizationError::RecoveryRequired)?;
            Ok(PreparedMergeEntry {
                entry,
                kind: object.kind(),
                bytes: object.data().to_vec(),
            })
        })
        .collect::<Result<Vec<_>, SynchronizationError>>()?;
    Ok(MergePreparation::Clean(PreparedMerge { entries }))
}

fn import_prepared_tree(
    repository: &git2::Repository,
    path: &Path,
    prepared: &PreparedMerge,
) -> Result<git2::Oid, SynchronizationError> {
    let _ = path; // The result bytes were captured while the worker mempack lived.
    let destination = repository
        .odb()
        .map_err(|_| SynchronizationError::RecoveryRequired)?;
    let mut index = git2::Index::new().map_err(|_| SynchronizationError::RecoveryRequired)?;
    for prepared_entry in &prepared.entries {
        if destination
            .write(prepared_entry.kind, &prepared_entry.bytes)
            .map_err(|_| SynchronizationError::RecoveryRequired)?
            != prepared_entry.entry.id
        {
            return Err(SynchronizationError::RecoveryRequired);
        }
        index
            .add(&prepared_entry.entry)
            .map_err(|_| SynchronizationError::RecoveryRequired)?;
    }
    index
        .write_tree_to(repository)
        .map_err(|_| SynchronizationError::RecoveryRequired)
}

/// Descriptor-observed bytes and filesystem identity of a regular index artifact.
#[cfg(any(unix, windows))]
struct IndexFileImage {
    bytes: Vec<u8>,
    device: u64,
    inode: u64,
}

#[cfg(unix)]
fn index_file_image_at(
    parent: &std::fs::File,
    name: &CString,
) -> Result<(std::fs::File, IndexFileImage), SynchronizationError> {
    let fd = unsafe {
        libc::openat(
            parent.as_raw_fd(),
            name.as_ptr(),
            libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK,
        )
    };
    if fd < 0 {
        return Err(SynchronizationError::ExternalChange);
    }
    let mut file = unsafe { std::fs::File::from_raw_fd(fd) };
    let metadata = file
        .metadata()
        .map_err(|_| SynchronizationError::ExternalChange)?;
    if !metadata.is_file() || metadata.mode() & 0o111 != 0 {
        return Err(SynchronizationError::ExternalChange);
    }
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)
        .map_err(|_| SynchronizationError::ExternalChange)?;
    file.seek(SeekFrom::Start(0))
        .map_err(|_| SynchronizationError::ExternalChange)?;
    Ok((
        file,
        IndexFileImage {
            bytes,
            device: metadata.dev(),
            inode: metadata.ino(),
        },
    ))
}

#[cfg(any(unix, windows))]
fn index_image_is_exact(observed: &IndexFileImage, expected: &IndexFileImage) -> bool {
    observed.device == expected.device
        && observed.inode == expected.inode
        && observed.bytes == expected.bytes
}

#[cfg(any(unix, windows))]
fn index_entries_match(left: &git2::Index, right: &git2::Index) -> bool {
    left.len() == right.len()
        && left.iter().zip(right.iter()).all(|(left, right)| {
            left.id == right.id && left.mode == right.mode && left.path == right.path
        })
}

#[cfg(any(unix, windows))]
fn approved_index_extensions(raw: &[u8]) -> Result<(), SynchronizationError> {
    const HEADER_LEN: usize = 12;
    const ENTRY_PREFIX_LEN: usize = 62;
    const CHECKSUM_LEN: usize = 20;
    if raw.len() < HEADER_LEN + CHECKSUM_LEN
        || &raw[..4] != b"DIRC"
        || u32::from_be_bytes(raw[4..8].try_into().expect("fixed index header")) != 2
    {
        return Err(SynchronizationError::ExternalChange);
    }
    let checksum_start = raw.len() - CHECKSUM_LEN;
    let entries = u32::from_be_bytes(raw[8..12].try_into().expect("fixed index header"));
    let mut offset = HEADER_LEN;
    for _ in 0..entries {
        let entry_start = offset;
        if offset + ENTRY_PREFIX_LEN > checksum_start {
            return Err(SynchronizationError::ExternalChange);
        }
        let flags = u16::from_be_bytes(
            raw[offset + 60..offset + ENTRY_PREFIX_LEN]
                .try_into()
                .expect("fixed index entry flags"),
        );
        offset += ENTRY_PREFIX_LEN;
        if flags & 0x4000 != 0 {
            if offset + 2 > checksum_start {
                return Err(SynchronizationError::ExternalChange);
            }
            offset += 2;
        }
        let Some(nul) = raw[offset..checksum_start]
            .iter()
            .position(|byte| *byte == 0)
        else {
            return Err(SynchronizationError::ExternalChange);
        };
        offset += nul + 1;
        offset = entry_start + (offset - entry_start).next_multiple_of(8);
        if offset > checksum_start {
            return Err(SynchronizationError::ExternalChange);
        }
    }
    while offset < checksum_start {
        if offset + 8 > checksum_start {
            return Err(SynchronizationError::ExternalChange);
        }
        let signature = &raw[offset..offset + 4];
        let length = u32::from_be_bytes(
            raw[offset + 4..offset + 8]
                .try_into()
                .expect("fixed index extension length"),
        ) as usize;
        offset += 8;
        let Some(end) = offset.checked_add(length) else {
            return Err(SynchronizationError::ExternalChange);
        };
        if end > checksum_start {
            return Err(SynchronizationError::ExternalChange);
        }
        // Git's lowercase signatures are required extensions, while uppercase
        // signatures are optional. Locked libgit2 preserves REUC/NAME semantics;
        // these advisory caches may be normalized or dropped. All other formats
        // remain fail-closed before canonical writes.
        let permitted = matches!(
            signature,
            b"REUC" | b"NAME" | b"TREE" | b"UNTR" | b"FSMN" | b"EOIE" | b"IEOT"
        );
        if !permitted {
            return Err(SynchronizationError::ExternalChange);
        }
        offset = end;
    }
    Ok(())
}

#[cfg(unix)]
fn index_leaf_present(
    parent: &std::fs::File,
    name: &CString,
) -> Result<bool, SynchronizationError> {
    let mut metadata = std::mem::MaybeUninit::<libc::stat>::uninit();
    let result = unsafe {
        libc::fstatat(
            parent.as_raw_fd(),
            name.as_ptr(),
            metadata.as_mut_ptr(),
            libc::AT_SYMLINK_NOFOLLOW,
        )
    };
    if result == 0 {
        Ok(true)
    } else if std::io::Error::last_os_error().kind() == std::io::ErrorKind::NotFound {
        Ok(false)
    } else {
        Err(SynchronizationError::ExternalChange)
    }
}

#[cfg(unix)]
fn index_from_private_path(
    file: &std::fs::File,
    path: &Path,
) -> Result<git2::Index, SynchronizationError> {
    let parent_path = path.parent().ok_or(SynchronizationError::ExternalChange)?;
    let parent = native_resolution::open_directory(parent_path)
        .map_err(|_| SynchronizationError::ExternalChange)?;
    let name = CString::new(
        path.file_name()
            .ok_or(SynchronizationError::ExternalChange)?
            .as_bytes(),
    )
    .map_err(|_| SynchronizationError::ExternalChange)?;
    let stamp = ref_log_stamp(file)?;
    let (current, _) = index_file_image_at(&parent, &name)?;
    if ref_log_stamp(&current)? != stamp {
        return Err(SynchronizationError::ExternalChange);
    }
    // Stock libgit2 requires a pathname. Cooperative private namespace plus
    // descriptor pre/post proof works on macOS too; no /proc filesystem required.
    let index = git2::Index::open(path).map_err(|_| SynchronizationError::ExternalChange)?;
    let (current, _) = index_file_image_at(&parent, &name)?;
    if ref_log_stamp(file)? != stamp
        || ref_log_stamp(&current)? != stamp
        || !native_resolution::directory_matches(parent_path, &parent)
            .map_err(|_| SynchronizationError::ExternalChange)?
    {
        return Err(SynchronizationError::ExternalChange);
    }
    Ok(index)
}

#[cfg(unix)]
fn fixed_index_name(name: &str) -> CString {
    CString::new(name).expect("fixed artifact role")
}

#[cfg(unix)]
fn open_index_directory(
    parent: &std::fs::File,
    name: &CString,
) -> Result<std::fs::File, SynchronizationError> {
    native_resolution::open_directory_at(parent, name)
        .map_err(|_| SynchronizationError::ExternalChange)
}

#[cfg(unix)]
fn write_private_index_file(
    parent: &std::fs::File,
    name: &str,
    bytes: &[u8],
) -> Result<(), SynchronizationError> {
    let name = fixed_index_name(name);
    let fd = unsafe {
        libc::openat(
            parent.as_raw_fd(),
            name.as_ptr(),
            libc::O_WRONLY | libc::O_CREAT | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK,
            0o600,
        )
    };
    if fd < 0 {
        return Err(SynchronizationError::ExternalChange);
    }
    let mut file = unsafe { std::fs::File::from_raw_fd(fd) };
    let metadata = file
        .metadata()
        .map_err(|_| SynchronizationError::ExternalChange)?;
    if !metadata.is_file() || metadata.mode() & 0o111 != 0 {
        return Err(SynchronizationError::ExternalChange);
    }
    file.set_len(0)
        .and_then(|()| file.write_all(bytes))
        .and_then(|()| file.sync_all())
        .map_err(|_| SynchronizationError::ExternalChange)
}

#[cfg(unix)]
fn backend_lock_present_at(
    parent: &std::fs::File,
    relative: &Path,
) -> Result<bool, SynchronizationError> {
    let mut components = relative.components().peekable();
    let mut directory = parent
        .try_clone()
        .map_err(|_| SynchronizationError::RecoveryRequired)?;
    while let Some(component) = components.next() {
        let std::path::Component::Normal(name) = component else {
            return Err(SynchronizationError::RecoveryRequired);
        };
        let name =
            CString::new(name.as_bytes()).map_err(|_| SynchronizationError::RecoveryRequired)?;
        if components.peek().is_none() {
            return index_leaf_present(&directory, &name)
                .map_err(|_| SynchronizationError::RecoveryRequired);
        }
        if !index_leaf_present(&directory, &name)
            .map_err(|_| SynchronizationError::RecoveryRequired)?
        {
            return Ok(false);
        }
        directory = open_index_directory(&directory, &name)
            .map_err(|_| SynchronizationError::RecoveryRequired)?;
    }
    Err(SynchronizationError::RecoveryRequired)
}

/// Relevant files-backend lock roles have no application-created durable
/// provenance. In particular, a loose-ref lock may guard partial reflog appends.
#[cfg(unix)]
fn refuse_ambiguous_resolution_backend_locks(
    repository: &git2::Repository,
) -> Result<(), SynchronizationError> {
    let branch = repository
        .head()
        .map_err(|_| SynchronizationError::RecoveryRequired)?
        .name()
        .ok_or(SynchronizationError::RecoveryRequired)?
        .to_owned();
    if !branch.starts_with("refs/heads/") {
        return Err(SynchronizationError::RecoveryRequired);
    }
    for (root, roles) in [
        (
            repository.path(),
            vec![PathBuf::from("HEAD.lock"), PathBuf::from("logs/HEAD.lock")],
        ),
        (
            repository.commondir(),
            vec![
                PathBuf::from("packed-refs.lock"),
                PathBuf::from(format!("{branch}.lock")),
                PathBuf::from(format!("logs/{branch}.lock")),
            ],
        ),
    ] {
        let parent = open_ref_log_root(root)?;
        for role in roles {
            if backend_lock_present_at(&parent, &role)? {
                return Err(SynchronizationError::RecoveryRequired);
            }
        }
    }
    Ok(())
}

#[cfg(any(unix, windows))]
const RESOLUTION_REF_MESSAGE: &str = "manyhands resolution";

#[cfg(unix)]
fn open_ref_log_root(path: &Path) -> Result<std::fs::File, SynchronizationError> {
    native_resolution::open_directory(path).map_err(|_| SynchronizationError::RecoveryRequired)
}

#[cfg(unix)]
fn ref_root_identity(path: &Path) -> Result<[u64; 2], SynchronizationError> {
    let meta = open_ref_log_root(path)?
        .metadata()
        .map_err(|_| SynchronizationError::RecoveryRequired)?;
    Ok([meta.dev(), meta.ino()])
}

#[cfg(unix)]
fn open_ref_log_file(
    root: &Path,
    role: &Path,
) -> Result<Option<std::fs::File>, SynchronizationError> {
    let mut parent = open_ref_log_root(root)?;
    let mut components = role.components().peekable();
    while let Some(component) = components.next() {
        let std::path::Component::Normal(name) = component else {
            return Err(SynchronizationError::RecoveryRequired);
        };
        let name =
            CString::new(name.as_bytes()).map_err(|_| SynchronizationError::RecoveryRequired)?;
        if !index_leaf_present(&parent, &name)
            .map_err(|_| SynchronizationError::RecoveryRequired)?
        {
            return Ok(None);
        }
        if components.peek().is_some() {
            parent = open_index_directory(&parent, &name)
                .map_err(|_| SynchronizationError::RecoveryRequired)?;
        } else {
            // NONBLOCK prevents FIFO/device reads from hanging before fstat refusal.
            let fd = unsafe {
                libc::openat(
                    parent.as_raw_fd(),
                    name.as_ptr(),
                    libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK,
                )
            };
            if fd < 0 {
                return Err(SynchronizationError::RecoveryRequired);
            }
            let file = unsafe { std::fs::File::from_raw_fd(fd) };
            ref_log_stamp(&file)?;
            return Ok(Some(file));
        }
    }
    Err(SynchronizationError::RecoveryRequired)
}

#[cfg(any(unix, windows))]
type RefLogStamp = (u64, u64, u64, u32, i64, i64, i64, i64);

/// Ordinary file and containing-directory barriers for a fixed Git role. An
/// absent loose object may already live in a pack/alternate; that backend's
/// storage ordering remains governed by its existing fsync policy. No config
/// or global libgit2 option is changed, and no exhaustive power-loss guarantee
/// (including macOS full device-cache flush) is inferred from these barriers.
#[cfg(unix)]
fn sync_git_role(root: &Path, role: &Path) -> Result<(), SynchronizationError> {
    if let Some(file) = open_ref_log_file(root, role)? {
        file.sync_all()
            .map_err(|_| SynchronizationError::RecoveryRequired)?;
    }
    let mut directories = vec![open_ref_log_root(root)?];
    let parent = role
        .parent()
        .ok_or(SynchronizationError::RecoveryRequired)?;
    for component in parent.components() {
        let std::path::Component::Normal(name) = component else {
            return Err(SynchronizationError::RecoveryRequired);
        };
        let name =
            CString::new(name.as_bytes()).map_err(|_| SynchronizationError::RecoveryRequired)?;
        let directory = directories
            .last()
            .ok_or(SynchronizationError::RecoveryRequired)?;
        if !index_leaf_present(directory, &name)? {
            break;
        }
        directories.push(open_index_directory(directory, &name)?);
    }
    for directory in directories.iter().rev() {
        directory
            .sync_all()
            .map_err(|_| SynchronizationError::RecoveryRequired)?;
    }
    Ok(())
}

#[cfg(any(unix, windows))]
fn sync_resolution_candidate(
    repository: &git2::Repository,
    candidate: git2::Oid,
    resolutions: &[(merge::ConflictPathToken, merge::RedactedConflictBytes)],
) -> Result<(), SynchronizationError> {
    let commit = repository
        .find_commit(candidate)
        .map_err(|_| SynchronizationError::RecoveryRequired)?;
    let tree = commit
        .tree()
        .map_err(|_| SynchronizationError::RecoveryRequired)?;
    let mut objects = std::collections::BTreeSet::from([candidate, tree.id()]);
    // Tree serialization can create nested tree objects, not just the root.
    tree.walk(git2::TreeWalkMode::PostOrder, |_, entry| {
        if entry.kind() == Some(git2::ObjectType::Tree) {
            objects.insert(entry.id());
        }
        git2::TreeWalkResult::Ok
    })
    .map_err(|_| SynchronizationError::RecoveryRequired)?;
    for (token, _) in resolutions {
        let path =
            std::str::from_utf8(&token.path).map_err(|_| SynchronizationError::RecoveryRequired)?;
        objects.insert(
            tree.get_path(Path::new(path))
                .map_err(|_| SynchronizationError::RecoveryRequired)?
                .id(),
        );
    }
    for oid in objects {
        let oid = oid.to_string();
        sync_git_role(
            repository.commondir(),
            Path::new(&format!("objects/{}/{}", &oid[..2], &oid[2..])),
        )?;
    }
    Ok(())
}

#[cfg(unix)]
fn ref_log_stamp(file: &std::fs::File) -> Result<RefLogStamp, SynchronizationError> {
    let meta = file
        .metadata()
        .map_err(|_| SynchronizationError::RecoveryRequired)?;
    if !meta.is_file() || meta.mode() & 0o111 != 0 {
        return Err(SynchronizationError::RecoveryRequired);
    }
    Ok((
        meta.dev(),
        meta.ino(),
        meta.len(),
        meta.mode(),
        meta.mtime(),
        meta.mtime_nsec(),
        meta.ctime(),
        meta.ctime_nsec(),
    ))
}

#[cfg(any(unix, windows))]
#[derive(Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct RefLogImage {
    present: bool,
    length: u64,
    digest: [u8; 32],
    // Consistent observation, NOT live lock ownership or required operator inode.
    identity: Option<[u64; 2]>,
}

#[cfg(any(unix, windows))]
impl RefLogImage {
    fn same_image(&self, other: &Self) -> bool {
        self.present == other.present && self.length == other.length && self.digest == other.digest
    }
}

#[cfg(any(unix, windows))]
struct RefLogObservation {
    #[cfg(windows)]
    namespace: native_resolution::Directory,
    file: Option<std::fs::File>,
    stamp: Option<RefLogStamp>,
    hash: blake3::Hasher,
    image: RefLogImage,
}

#[cfg(any(unix, windows))]
impl RefLogObservation {
    fn read(root: &Path, role: &Path) -> Result<Self, SynchronizationError> {
        #[cfg(windows)]
        let namespace = windows_resolution::observation_namespace(root, role)?;
        let mut file = open_ref_log_file(root, role)?;
        let stamp = file.as_ref().map(ref_log_stamp).transpose()?;
        let mut hash = blake3::Hasher::new();
        if let Some(file) = &mut file {
            let mut buffer = [0; 65536];
            loop {
                let length = file
                    .read(&mut buffer)
                    .map_err(|_| SynchronizationError::RecoveryRequired)?;
                if length == 0 {
                    break;
                }
                hash.update(&buffer[..length]);
            }
            if Some(ref_log_stamp(file)?) != stamp {
                return Err(SynchronizationError::RecoveryRequired);
            }
        }
        let image = RefLogImage {
            present: file.is_some(),
            length: stamp.map_or(0, |s| s.2),
            digest: *hash.finalize().as_bytes(),
            identity: stamp.map(|s| [s.0, s.1]),
        };
        Ok(Self {
            #[cfg(windows)]
            namespace,
            file,
            stamp,
            hash,
            image,
        })
    }
    fn revalidate(&self, root: &Path, role: &Path) -> Result<(), SynchronizationError> {
        #[cfg(windows)]
        self.namespace
            .revalidate()
            .map_err(|_| SynchronizationError::RecoveryRequired)?;
        let current = open_ref_log_file(root, role)?;
        if current.as_ref().map(ref_log_stamp).transpose()? != self.stamp
            || self.file.as_ref().map(ref_log_stamp).transpose()? != self.stamp
        {
            return Err(SynchronizationError::RecoveryRequired);
        }
        Ok(())
    }
    fn intended(&self, entry: Option<&[u8]>) -> RefLogImage {
        let mut image = self.image.clone();
        if let Some(entry) = entry {
            let mut hash = self.hash.clone();
            hash.update(entry);
            image.present = true;
            image.length += entry.len() as u64;
            image.digest = *hash.finalize().as_bytes();
        }
        image
    }
}

#[cfg(any(unix, windows))]
fn resolution_log_policy(repository: &git2::Repository) -> Result<u8, SynchronizationError> {
    let config = repository
        .config()
        .map_err(|_| SynchronizationError::RecoveryRequired)?;
    // Files backend only. Unknown policy is rejected, not defaulted to logging.
    match config.get_string("extensions.refstorage") {
        Err(error) if error.code() == git2::ErrorCode::NotFound => {}
        Ok(value) if value == "files" => {}
        _ => return Err(SynchronizationError::RecoveryRequired),
    }
    match config.get_string("core.logallrefupdates") {
        Err(error) if error.code() == git2::ErrorCode::NotFound && !repository.is_bare() => Ok(1),
        Ok(value) if value == "always" => Ok(3),
        Ok(_) => match config.get_bool("core.logallrefupdates") {
            Ok(false) => Ok(0),
            Ok(true) => Ok(2),
            _ => Err(SynchronizationError::RecoveryRequired),
        },
        _ => Err(SynchronizationError::RecoveryRequired),
    }
}

#[cfg(any(unix, windows))]
struct RefLogSnapshot {
    branch: String,
    policy: u8,
    roots: [[u64; 2]; 2],
    logs: [RefLogObservation; 2], // fixed roles: common branch, target-worktree HEAD
}

#[cfg(any(unix, windows))]
impl RefLogSnapshot {
    fn storage_barrier(&self, repository: &git2::Repository) -> Result<(), SynchronizationError> {
        self.revalidate(repository)?;
        for (root, role) in [
            (repository.commondir(), PathBuf::from(&self.branch)),
            (repository.commondir(), PathBuf::from("packed-refs")),
            (repository.path(), PathBuf::from("HEAD")),
            (
                repository.commondir(),
                PathBuf::from(format!("logs/{}", self.branch)),
            ),
            (repository.path(), PathBuf::from("logs/HEAD")),
        ] {
            sync_git_role(root, &role)?;
        }
        self.revalidate(repository)
    }

    fn read(repository: &git2::Repository) -> Result<Self, SynchronizationError> {
        let head = repository
            .find_reference("HEAD")
            .map_err(|_| SynchronizationError::RecoveryRequired)?;
        let branch = head
            .symbolic_target()
            .ok_or(SynchronizationError::RecoveryRequired)?
            .to_owned();
        if !branch.starts_with("refs/heads/")
            || !git2::Reference::is_valid_name(&branch)
            || branch.as_bytes().iter().any(|c| c.is_ascii_control())
        {
            return Err(SynchronizationError::RecoveryRequired);
        }
        if repository
            .find_reference(&branch)
            .ok()
            .and_then(|reference| reference.target())
            .is_none()
        {
            return Err(SynchronizationError::RecoveryRequired);
        }
        for (root, role) in [
            (repository.commondir(), Path::new(branch.as_str())),
            (repository.commondir(), Path::new("packed-refs")),
            (repository.path(), Path::new("HEAD")),
        ] {
            open_ref_log_file(root, role)?;
        }
        let policy = resolution_log_policy(repository)?;
        refuse_ambiguous_resolution_backend_locks(repository)?;
        let roots = [repository.commondir(), repository.path()]
            .map(ref_root_identity)
            .into_iter()
            .collect::<Result<Vec<_>, SynchronizationError>>()?
            .try_into()
            .map_err(|_| SynchronizationError::RecoveryRequired)?;
        let logs = [
            RefLogObservation::read(repository.commondir(), Path::new(&format!("logs/{branch}")))?,
            RefLogObservation::read(repository.path(), Path::new("logs/HEAD"))?,
        ];
        Ok(Self {
            branch,
            policy,
            roots,
            logs,
        })
    }
    fn revalidate(&self, repository: &git2::Repository) -> Result<(), SynchronizationError> {
        if repository
            .find_reference("HEAD")
            .ok()
            .and_then(|head| head.symbolic_target().map(str::to_owned))
            .as_deref()
            != Some(&self.branch)
            || resolution_log_policy(repository)? != self.policy
            || repository
                .find_reference(&self.branch)
                .ok()
                .and_then(|reference| reference.target())
                .is_none()
        {
            return Err(SynchronizationError::RecoveryRequired);
        }
        for (root, role) in [
            (repository.commondir(), Path::new(self.branch.as_str())),
            (repository.commondir(), Path::new("packed-refs")),
            (repository.path(), Path::new("HEAD")),
        ] {
            open_ref_log_file(root, role)?;
        }
        for (index, root) in [repository.commondir(), repository.path()]
            .iter()
            .enumerate()
        {
            if ref_root_identity(root)? != self.roots[index] {
                return Err(SynchronizationError::RecoveryRequired);
            }
        }
        self.logs[0].revalidate(
            repository.commondir(),
            Path::new(&format!("logs/{}", self.branch)),
        )?;
        self.logs[1].revalidate(repository.path(), Path::new("logs/HEAD"))
    }
    fn matches(&self, images: &[RefLogImage; 2]) -> bool {
        self.logs
            .iter()
            .zip(images)
            .all(|(log, image)| log.image.same_image(image))
    }

    // Caller holds the reacquired common-Git lease. A refreshed snapshot must
    // authenticate the frozen target, not merely its own current branch/images.
    fn validate_final_proof(
        &self,
        repository: &git2::Repository,
        proof: &RefLogManifest,
        candidate: git2::Oid,
    ) -> Result<(), SynchronizationError> {
        self.revalidate(repository)?;
        if !self.matches(
            proof
                .intended
                .as_ref()
                .ok_or(SynchronizationError::RecoveryRequired)?,
        ) || self.policy != proof.policy
            || self.roots != proof.roots
            || *blake3::hash(self.branch.as_bytes()).as_bytes() != proof.branch_digest
            || repository
                .refname_to_id("HEAD")
                .map_err(|_| SynchronizationError::RecoveryRequired)?
                != candidate
        {
            return Err(SynchronizationError::RecoveryRequired);
        }
        Ok(())
    }
}

#[cfg(any(unix, windows))]
fn resolution_signer_digest(
    signature: &git2::Signature<'_>,
) -> Result<[u8; 32], SynchronizationError> {
    let name = signature
        .name()
        .ok_or(SynchronizationError::RecoveryRequired)?;
    let email = signature
        .email()
        .ok_or(SynchronizationError::RecoveryRequired)?;
    if [name, email].iter().any(|s| {
        s.is_empty() || s.trim() != *s || s.chars().any(|c| c.is_control() || c == '<' || c == '>')
    }) || signature.when().seconds() != 0
        || signature.when().offset_minutes() != 0
    {
        return Err(SynchronizationError::RecoveryRequired);
    }
    let mut hash = blake3::Hasher::new();
    hash.update(b"manyhands-resolution-ref-signer-v1\0");
    preflight_bytes(&mut hash, name.as_bytes());
    preflight_bytes(&mut hash, email.as_bytes());
    hash.update(&0i64.to_be_bytes());
    hash.update(&0i32.to_be_bytes());
    Ok(*hash.finalize().as_bytes())
}

#[cfg(any(unix, windows))]
#[derive(Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct RefLogManifest {
    version: u8,
    operation: String,
    attempt: String,
    ordinal: u8,
    observation: [u8; 32],
    input: [u8; 32],
    old: [u8; 20],
    candidate: Option<[u8; 20]>,
    sentinel_identity: [u64; 2],
    sentinel_digest: [u8; 32],
    branch_digest: [u8; 32],
    roots: [[u64; 2]; 2],
    policy: u8,
    signer_digest: [u8; 32],
    message_digest: [u8; 32],
    baseline: [RefLogImage; 2],
    intended: Option<[RefLogImage; 2]>,
    baseline_manifest_digest: Option<[u8; 32]>,
}

#[cfg(any(unix, windows))]
#[derive(Clone, Copy)]
struct RefLogContext<'a> {
    service: &'a RepositoryService,
    root: &'a Path,
    owner: &'a RemoteReservation,
    request: &'a merge::ResolveSynchronizationRequest,
    input: [u8; 32],
}

#[cfg(unix)]
fn ref_manifest_image_at(
    parent: &std::fs::File,
    name: &CString,
) -> Result<IndexFileImage, SynchronizationError> {
    let fd = unsafe {
        libc::openat(
            parent.as_raw_fd(),
            name.as_ptr(),
            libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK,
        )
    };
    if fd < 0 {
        return Err(SynchronizationError::RecoveryRequired);
    }
    let mut file = unsafe { std::fs::File::from_raw_fd(fd) };
    let stamp = ref_log_stamp(&file)?;
    if stamp.2 > 16384 || stamp.3 & 0o777 != 0o600 {
        return Err(SynchronizationError::RecoveryRequired);
    }
    let mut bytes = Vec::new();
    (&mut file)
        .take(16385)
        .read_to_end(&mut bytes)
        .map_err(|_| SynchronizationError::RecoveryRequired)?;
    if ref_log_stamp(&file)? != stamp || bytes.len() as u64 != stamp.2 {
        return Err(SynchronizationError::RecoveryRequired);
    }
    Ok(IndexFileImage {
        bytes,
        device: stamp.0,
        inode: stamp.1,
    })
}

#[cfg(unix)]
impl ResolutionIndexLock {
    fn ref_manifest(
        &self,
        service: &RepositoryService,
        root: &Path,
        owner: &RemoteReservation,
        attempt: OperationId,
        role: &str,
    ) -> Result<Option<(RefLogManifest, state::ResolutionRefLogArtifact)>, SynchronizationError>
    {
        let artifact = state::with_transaction(service, root, |tx, id| {
            let record = state::read_operation(tx, id, owner.operation_id())?
                .ok_or_else(state::recovery_required)?;
            state::resolution_ref_log_artifact(tx, &record, attempt, role)
        })?;
        let Some(artifact) = artifact else {
            return Ok(None);
        };
        let name = format!("ref-log-{role}");
        let anchor_name = format!("{name}-anchor");
        let image = ref_manifest_image_at(&self.staging, &fixed_index_name(&name))?;
        let anchor = ref_manifest_image_at(&self.staging, &fixed_index_name(&anchor_name))?;
        if !index_image_is_exact(&image, &anchor)
            || image.device != artifact.device
            || image.inode != artifact.inode
            || *blake3::hash(&image.bytes).as_bytes() != artifact.digest
        {
            return Err(SynchronizationError::RecoveryRequired);
        }
        let manifest = serde_yaml::from_slice(&image.bytes)
            .map_err(|_| SynchronizationError::RecoveryRequired)?;
        Ok(Some((manifest, artifact)))
    }
    fn persist_ref_manifest(
        &self,
        service: &RepositoryService,
        root: &Path,
        owner: &RemoteReservation,
        attempt: OperationId,
        role: &str,
        manifest: &RefLogManifest,
    ) -> Result<(), SynchronizationError> {
        if let Some((existing, _)) = self.ref_manifest(service, root, owner, attempt, role)? {
            return if existing == *manifest {
                Ok(())
            } else {
                Err(SynchronizationError::RecoveryRequired)
            };
        }
        // Until provenance is journaled, only these fixed private preparation
        // roles are disposable under the approved cooperative staging convention.
        let name = format!("ref-log-{role}");
        let anchor = format!("{name}-anchor");
        for role in [&name, &anchor] {
            let role = fixed_index_name(role);
            if index_leaf_present(&self.staging, &role)? {
                ref_manifest_image_at(&self.staging, &role)?;
                if unsafe { libc::unlinkat(self.staging.as_raw_fd(), role.as_ptr(), 0) } != 0 {
                    return Err(SynchronizationError::RecoveryRequired);
                }
            }
        }
        let bytes = serde_yaml::to_string(manifest)
            .map_err(|_| SynchronizationError::RecoveryRequired)?
            .into_bytes();
        write_private_index_file(&self.staging, &name, &bytes)?;
        if unsafe {
            libc::linkat(
                self.staging.as_raw_fd(),
                fixed_index_name(&name).as_ptr(),
                self.staging.as_raw_fd(),
                fixed_index_name(&anchor).as_ptr(),
                0,
            )
        } != 0
        {
            return Err(SynchronizationError::RecoveryRequired);
        }
        self.staging
            .sync_all()
            .and_then(|()| self.parent.sync_all())
            .map_err(|_| SynchronizationError::RecoveryRequired)?;
        let (_, image) = index_file_image_at(&self.staging, &fixed_index_name(&name))?;
        service.prepare_synchronization_ref_log_artifact(
            root,
            owner,
            attempt,
            role,
            &state::ResolutionRefLogArtifact {
                device: image.device,
                inode: image.inode,
                digest: *blake3::hash(&bytes).as_bytes(),
            },
        )?;
        self.ref_manifest(service, root, owner, attempt, role)?
            .ok_or(SynchronizationError::RecoveryRequired)?;
        Ok(())
    }
}

#[cfg(any(unix, windows))]
impl ResolutionIndexLock {
    fn bind_ref_baseline(
        &self,
        context: RefLogContext<'_>,
        snapshot: &RefLogSnapshot,
        signature: &git2::Signature<'_>,
    ) -> Result<(), SynchronizationError> {
        let RefLogContext {
            service,
            root,
            owner,
            request,
            input,
        } = context;
        let manifest = RefLogManifest {
            version: 1,
            operation: owner.operation_id().to_string(),
            attempt: request.attempt_id.to_string(),
            ordinal: request.observation.ordinal,
            observation: request.observation.fingerprint,
            input,
            old: request
                .observation
                .head
                .as_bytes()
                .try_into()
                .map_err(|_| SynchronizationError::RecoveryRequired)?,
            candidate: None,
            sentinel_identity: [self.artifact.device, self.artifact.inode],
            sentinel_digest: self.artifact.sentinel_digest,
            branch_digest: *blake3::hash(snapshot.branch.as_bytes()).as_bytes(),
            roots: snapshot.roots,
            policy: snapshot.policy,
            signer_digest: resolution_signer_digest(signature)?,
            message_digest: *blake3::hash(RESOLUTION_REF_MESSAGE.as_bytes()).as_bytes(),
            baseline: snapshot.logs.each_ref().map(|log| log.image.clone()),
            intended: None,
            baseline_manifest_digest: None,
        };
        if let Some((existing, _)) =
            self.ref_manifest(service, root, owner, request.attempt_id, "baseline")?
        {
            let mut expected = manifest;
            expected.baseline = existing.baseline.clone(); // Restored images need not retain old inode.
            if existing != expected || !snapshot.matches(&existing.baseline) {
                return Err(SynchronizationError::RecoveryRequired);
            }
            return Ok(());
        }
        self.persist_ref_manifest(
            service,
            root,
            owner,
            request.attempt_id,
            "baseline",
            &manifest,
        )
    }
    fn bind_ref_transition(
        &self,
        context: RefLogContext<'_>,
        candidate: git2::Oid,
        snapshot: &RefLogSnapshot,
        signature: &git2::Signature<'_>,
    ) -> Result<RefLogManifest, SynchronizationError> {
        let RefLogContext {
            service,
            root,
            owner,
            request,
            input,
        } = context;
        let (baseline, provenance) = self
            .ref_manifest(service, root, owner, request.attempt_id, "baseline")?
            .ok_or(SynchronizationError::RecoveryRequired)?;
        if baseline.version != 1
            || baseline.operation != owner.operation_id().to_string()
            || baseline.attempt != request.attempt_id.to_string()
            || baseline.ordinal != request.observation.ordinal
            || baseline.observation != request.observation.fingerprint
            || baseline.input != input
            || baseline.old.as_slice() != request.observation.head.as_bytes()
            || baseline.candidate.is_some()
            || baseline.intended.is_some()
            || baseline.baseline_manifest_digest.is_some()
            || baseline.sentinel_identity != [self.artifact.device, self.artifact.inode]
            || baseline.sentinel_digest != self.artifact.sentinel_digest
            || baseline.branch_digest != *blake3::hash(snapshot.branch.as_bytes()).as_bytes()
            || baseline.roots != snapshot.roots
            || baseline.policy != snapshot.policy
            || baseline.signer_digest != resolution_signer_digest(signature)?
            || baseline.message_digest
                != *blake3::hash(RESOLUTION_REF_MESSAGE.as_bytes()).as_bytes()
        {
            return Err(SynchronizationError::RecoveryRequired);
        }
        if let Some((transition, _)) =
            self.ref_manifest(service, root, owner, request.attempt_id, "transition")?
        {
            let mut expected = baseline.clone();
            expected.candidate = Some(
                candidate
                    .as_bytes()
                    .try_into()
                    .map_err(|_| SynchronizationError::RecoveryRequired)?,
            );
            expected.intended = transition.intended.clone();
            expected.baseline_manifest_digest = Some(provenance.digest);
            if transition != expected || transition.intended.is_none() {
                return Err(SynchronizationError::RecoveryRequired);
            }
            return Ok(transition);
        }
        if self.artifact.ref_phase != "not_started" || !snapshot.matches(&baseline.baseline) {
            return Err(SynchronizationError::RecoveryRequired);
        }
        // EXPECTATION ONLY, never passed to any log writer. Locked source
        // serialize_reflog_entry and seven API characterization tests establish
        // this fixed-message, validated-identity, zero-time/UTC append format.
        let entry = format!(
            "{} {} {} <{}> 0 +0000\t{}\n",
            request.observation.head,
            candidate,
            signature
                .name()
                .ok_or(SynchronizationError::RecoveryRequired)?,
            signature
                .email()
                .ok_or(SynchronizationError::RecoveryRequired)?,
            RESOLUTION_REF_MESSAGE
        );
        let mut transition = baseline;
        transition.candidate = Some(
            candidate
                .as_bytes()
                .try_into()
                .map_err(|_| SynchronizationError::RecoveryRequired)?,
        );
        transition.intended = Some(
            snapshot
                .logs
                .each_ref()
                .map(|log| log.intended((snapshot.policy != 0).then_some(entry.as_bytes()))),
        );
        transition.baseline_manifest_digest = Some(provenance.digest);
        self.persist_ref_manifest(
            service,
            root,
            owner,
            request.attempt_id,
            "transition",
            &transition,
        )?;
        Ok(transition)
    }
}

#[cfg(any(unix, windows))]
fn apply_resolution_ref(
    repository: &git2::Repository,
    context: RefLogContext<'_>,
    candidate: git2::Oid,
    snapshot: &RefLogSnapshot,
    proof: &RefLogManifest,
) -> Result<(), SynchronizationError> {
    let RefLogContext {
        service,
        root,
        owner,
        request,
        ..
    } = context;
    refuse_ambiguous_resolution_backend_locks(repository)?;
    let mut transaction = repository
        .transaction()
        .map_err(|_| SynchronizationError::RecoveryRequired)?;
    transaction
        .lock_ref(&snapshot.branch)
        .map_err(|_| SynchronizationError::RecoveryRequired)?;
    #[cfg(test)]
    tests::process_death_boundary(root, "ref-locked");
    snapshot.revalidate(repository)?;
    if repository
        .refname_to_id(&snapshot.branch)
        .ok()
        .is_none_or(|oid| oid.as_bytes() != proof.old)
        || !snapshot.matches(&proof.baseline)
    {
        return Err(SynchronizationError::RecoveryRequired);
    }
    let commit = repository
        .find_commit(candidate)
        .map_err(|_| SynchronizationError::RecoveryRequired)?;
    let signature = commit.committer();
    if resolution_signer_digest(&signature)? != proof.signer_digest {
        return Err(SynchronizationError::RecoveryRequired);
    }
    let progress = state::with_transaction(service, root, |tx, id| {
        let record = state::read_operation(tx, id, owner.operation_id())?
            .ok_or_else(state::recovery_required)?;
        Ok(
            state::resolution_index_artifact(tx, &record, request.attempt_id)?
                .ok_or_else(state::recovery_required)?
                .ref_phase,
        )
    })?;
    // Operator restoration can restore baseline after ref observation but before
    // checkpoint observation. Keep progress monotonic; actual-state proof, not
    // this marker, authorizes the native effect. The wrapper still fences owner.
    service.advance_synchronization_resolution_ref_effect(
        root,
        owner,
        request.attempt_id,
        if progress == "observed" {
            "observed"
        } else {
            "intent"
        },
    )?;
    #[cfg(test)]
    tests::process_death_boundary(root, "ref-intent");
    transaction
        .set_target(
            &snapshot.branch,
            candidate,
            Some(&signature),
            RESOLUTION_REF_MESSAGE,
        )
        .map_err(|_| SynchronizationError::RecoveryRequired)?;
    transaction
        .commit()
        .map_err(|_| SynchronizationError::RecoveryRequired)?;
    #[cfg(test)]
    tests::process_death_boundary(root, "ref-committed");
    Ok(())
}

#[cfg(any(unix, windows))]
const RESOLUTION_MERGE_MEMBERS: [&str; 3] = ["MERGE_HEAD", "MERGE_MSG", "MERGE_MODE"];

/// Operation-private hard-link anchor and durable immutable identity establish
/// ownership, not sentinel contents. Never exchanged with or installed as index.
#[cfg(not(windows))]
struct ResolutionIndexLock {
    #[cfg(unix)]
    parent: std::fs::File,
    #[cfg(unix)]
    staging: std::fs::File,
    #[cfg(unix)]
    parent_path: PathBuf,
    #[cfg(unix)]
    staging_path: PathBuf,
    #[cfg(unix)]
    baseline: IndexFileImage,
    #[cfg(unix)]
    sentinel: IndexFileImage,
    #[cfg(unix)]
    artifact: state::ResolutionIndexArtifact,
    #[cfg(all(test, unix))]
    workdir: PathBuf,
}

#[cfg(not(windows))]
impl ResolutionIndexLock {
    #[cfg(unix)]
    fn acquire(
        repository: &git2::Repository,
        service: &RepositoryService,
        root: &Path,
        owner: &RemoteReservation,
        attempt: OperationId,
    ) -> Result<Self, SynchronizationError> {
        let parent_path = repository.path().to_owned();
        let parent = native_resolution::open_directory(&parent_path)
            .map_err(|_| SynchronizationError::ExternalChange)?;
        let retained = state::with_transaction(service, root, |tx, id| {
            let record = state::read_operation(tx, id, owner.operation_id())?
                .ok_or_else(state::recovery_required)?;
            state::resolution_index_artifact(tx, &record, attempt)
        })?;
        let stage_name = fixed_index_name(&format!(".manyhands-resolution-{attempt}"));
        let staging_path = parent_path.join(format!(".manyhands-resolution-{attempt}"));
        let lock_name = fixed_index_name("index.lock");
        if retained.is_none() && index_leaf_present(&parent, &lock_name)? {
            // No journaled anchor means no authority over any live lock.
            return Err(SynchronizationError::ExternalChange);
        }
        if retained.is_none()
            && !index_leaf_present(&parent, &stage_name)?
            && unsafe { libc::mkdirat(parent.as_raw_fd(), stage_name.as_ptr(), 0o700) } != 0
        {
            return Err(SynchronizationError::ExternalChange);
        }
        let staging = open_index_directory(&parent, &stage_name)?;
        if staging
            .metadata()
            .map_err(|_| SynchronizationError::ExternalChange)?
            .dev()
            != parent
                .metadata()
                .map_err(|_| SynchronizationError::ExternalChange)?
                .dev()
        {
            return Err(SynchronizationError::ExternalChange);
        }
        let artifact = if let Some(artifact) = retained {
            artifact
        } else {
            let (_, baseline) = index_file_image_at(&parent, &fixed_index_name("index"))?;
            approved_index_extensions(&baseline.bytes)?;
            // Locked libgit2 validates checksum and the supported v2 format before effects.
            write_private_index_file(&staging, "baseline", &baseline.bytes)?;
            let (file, _) = index_file_image_at(&staging, &fixed_index_name("baseline"))?;
            index_from_private_path(&file, &staging_path.join("baseline"))?;
            let sentinel_bytes = format!(
                "manyhands-resolution-sentinel-v2\n{}\n{attempt}\n",
                owner.operation_id()
            )
            .into_bytes();
            write_private_index_file(&staging, "sentinel", &sentinel_bytes)?;
            let (_, sentinel) = index_file_image_at(&staging, &fixed_index_name("sentinel"))?;
            let mut metadata = [None; 3];
            for (ordinal, name) in RESOLUTION_MERGE_MEMBERS.iter().enumerate() {
                let name = fixed_index_name(name);
                if index_leaf_present(&parent, &name)? {
                    metadata[ordinal] = Some(
                        *blake3::hash(&index_file_image_at(&parent, &name)?.1.bytes).as_bytes(),
                    );
                }
            }
            staging
                .sync_all()
                .and_then(|()| parent.sync_all())
                .map_err(|_| SynchronizationError::ExternalChange)?;
            let artifact = state::ResolutionIndexArtifact {
                device: sentinel.device,
                inode: sentinel.inode,
                sentinel_digest: *blake3::hash(&sentinel.bytes).as_bytes(),
                baseline_digest: *blake3::hash(&baseline.bytes).as_bytes(),
                baseline_identity: (baseline.device, baseline.inode),
                metadata,
                output: None,
                ref_phase: "not_started".into(),
                phase: "intent".into(),
            };
            service.prepare_synchronization_index_artifact(root, owner, attempt, &artifact)?;
            artifact
        };
        let (_, baseline) = index_file_image_at(&staging, &fixed_index_name("baseline"))?;
        let (_, sentinel) = index_file_image_at(&staging, &fixed_index_name("sentinel"))?;
        if *blake3::hash(&baseline.bytes).as_bytes() != artifact.baseline_digest
            || sentinel.device != artifact.device
            || sentinel.inode != artifact.inode
            || *blake3::hash(&sentinel.bytes).as_bytes() != artifact.sentinel_digest
        {
            return Err(SynchronizationError::ExternalChange);
        }
        let mut held = Self {
            parent,
            staging,
            parent_path,
            staging_path,
            baseline,
            sentinel,
            artifact,
            #[cfg(test)]
            workdir: repository
                .workdir()
                .ok_or(SynchronizationError::ExternalChange)?
                .to_owned(),
        };
        if held.artifact.output.is_none() {
            let (_, live_baseline) = index_file_image_at(&held.parent, &fixed_index_name("index"))?;
            if live_baseline.bytes != held.baseline.bytes
                || (live_baseline.device, live_baseline.inode) != held.artifact.baseline_identity
            {
                return Err(SynchronizationError::ExternalChange);
            }
        }
        if held.artifact.phase == "intent" {
            // Only publish into absence, or recognize this exact retained inode.
            if !index_leaf_present(&held.parent, &lock_name)?
                && unsafe {
                    libc::linkat(
                        held.staging.as_raw_fd(),
                        fixed_index_name("sentinel").as_ptr(),
                        held.parent.as_raw_fd(),
                        lock_name.as_ptr(),
                        0,
                    )
                } != 0
            {
                return Err(SynchronizationError::ExternalChange);
            }
            held.verify_sentinel()?;
            #[cfg(test)]
            tests::process_death_boundary(&held.workdir, "sentinel-linked");
            held.parent
                .sync_all()
                .and_then(|()| held.staging.sync_all())
                .map_err(|_| SynchronizationError::ExternalChange)?;
            #[cfg(test)]
            run_resolution_index_lock_hook(
                repository
                    .workdir()
                    .ok_or(SynchronizationError::ExternalChange)?,
            );
            held.verify_sentinel()?;
            let (_, current) = index_file_image_at(&held.parent, &fixed_index_name("index"))?;
            if current.bytes != held.baseline.bytes
                || (current.device, current.inode) != held.artifact.baseline_identity
            {
                return Err(SynchronizationError::ExternalChange);
            }
            service.advance_synchronization_index_artifact(root, owner, attempt, "published")?;
            held.artifact.phase = "published".into();
        } else if held.artifact.phase == "published"
            || index_leaf_present(&held.parent, &lock_name)?
        {
            held.verify_sentinel()?;
        }
        Ok(held)
    }

    #[cfg(not(unix))]
    fn acquire(
        _repository: &git2::Repository,
        _service: &RepositoryService,
        _root: &Path,
        _owner: &RemoteReservation,
        _attempt: OperationId,
    ) -> Result<Self, SynchronizationError> {
        Err(SynchronizationError::ExternalChange)
    }

    #[cfg(unix)]
    fn verify_namespace(&self) -> Result<(), SynchronizationError> {
        if native_resolution::directory_matches(&self.parent_path, &self.parent)
            .map_err(|_| SynchronizationError::ExternalChange)?
            && native_resolution::directory_matches(&self.staging_path, &self.staging)
                .map_err(|_| SynchronizationError::ExternalChange)?
        {
            Ok(())
        } else {
            Err(SynchronizationError::ExternalChange)
        }
    }

    #[cfg(unix)]
    fn verify_sentinel(&self) -> Result<(), SynchronizationError> {
        self.verify_namespace()?;
        let (_, anchor) = index_file_image_at(&self.staging, &fixed_index_name("sentinel"))?;
        let (_, live) = index_file_image_at(&self.parent, &fixed_index_name("index.lock"))?;
        if index_image_is_exact(&anchor, &self.sentinel)
            && index_image_is_exact(&live, &self.sentinel)
        {
            Ok(())
        } else {
            Err(SynchronizationError::ExternalChange)
        }
    }

    #[cfg(unix)]
    fn authoritative_index(&self) -> Result<git2::Index, SynchronizationError> {
        self.verify_sentinel()?;
        // Private preparation is disposable until output identity is journaled.
        // Rebuild solely the operation-derived, fixed private index role.
        if self.artifact.output.is_some() {
            return Err(SynchronizationError::RecoveryRequired);
        }
        let private_lock = fixed_index_name("index.lock");
        if index_leaf_present(&self.staging, &private_lock)? {
            index_file_image_at(&self.staging, &private_lock)?;
            if unsafe { libc::unlinkat(self.staging.as_raw_fd(), private_lock.as_ptr(), 0) } != 0 {
                return Err(SynchronizationError::ExternalChange);
            }
        }
        write_private_index_file(&self.staging, "index", &self.baseline.bytes)?;
        let (file, _) = index_file_image_at(&self.staging, &fixed_index_name("index"))?;
        index_from_private_path(&file, &self.staging_path.join("index"))
    }

    #[cfg(not(unix))]
    fn authoritative_index(&self) -> Result<git2::Index, SynchronizationError> {
        Err(SynchronizationError::ExternalChange)
    }

    #[cfg(unix)]
    fn persisted_images_match(&self) -> Result<(), SynchronizationError> {
        self.verify_sentinel()?;
        self.installed_image_matches()
    }

    #[cfg(unix)]
    fn installed_image_matches(&self) -> Result<(), SynchronizationError> {
        self.verify_namespace()?;
        let (_, image) = index_file_image_at(&self.parent, &fixed_index_name("index"))?;
        match self.artifact.output {
            Some((device, inode, digest))
                if image.device == device
                    && image.inode == inode
                    && *blake3::hash(&image.bytes).as_bytes() == digest =>
            {
                Ok(())
            }
            _ => Err(SynchronizationError::ExternalChange),
        }
    }

    #[cfg(not(unix))]
    fn persisted_images_match(&self) -> Result<(), SynchronizationError> {
        Err(SynchronizationError::ExternalChange)
    }

    #[cfg(unix)]
    fn persist(
        &mut self,
        source: &mut git2::Index,
        _repository: &git2::Repository,
        service: &RepositoryService,
        root: &Path,
        owner: &RemoteReservation,
        attempt: OperationId,
    ) -> Result<(), SynchronizationError> {
        self.verify_sentinel()?;
        if source.path() != Some(self.staging_path.join("index").as_path()) {
            return Err(SynchronizationError::ExternalChange);
        }
        let (_, original) = index_file_image_at(&self.parent, &fixed_index_name("index"))?;
        if original.bytes != self.baseline.bytes
            || (original.device, original.inode) != self.artifact.baseline_identity
        {
            return Err(SynchronizationError::ExternalChange);
        }
        // Serialize the authoritative copy with the locked backend. REUC/NAME
        // remain semantic; optional cache extensions may be normalized/dropped.
        source
            .write()
            .map_err(|_| SynchronizationError::RecoveryRequired)?;
        #[cfg(test)]
        tests::process_death_boundary(&self.workdir, "scratch-serialized");
        let (file, prepared) = index_file_image_at(&self.staging, &fixed_index_name("index"))?;
        file.sync_all()
            .and_then(|()| self.staging.sync_all())
            .map_err(|_| SynchronizationError::ExternalChange)?;
        #[cfg(test)]
        run_resolution_index_scratch_hook(
            _repository
                .workdir()
                .ok_or(SynchronizationError::ExternalChange)?,
        );
        self.verify_namespace()?;
        let parsed = index_from_private_path(&file, &self.staging_path.join("index"))?;
        if !index_entries_match(&parsed, source) || parsed.has_conflicts() {
            return Err(SynchronizationError::ExternalChange);
        }
        let output = (
            prepared.device,
            prepared.inode,
            *blake3::hash(&prepared.bytes).as_bytes(),
        );
        service.prepare_synchronization_index_output(root, owner, attempt, output)?;
        self.artifact.output = Some(output);
        #[cfg(test)]
        run_resolution_index_persist_hook(
            _repository
                .workdir()
                .ok_or(SynchronizationError::ExternalChange)?,
        );
        self.install_prepared()?;
        #[cfg(test)]
        run_resolution_index_install_hook(
            _repository
                .workdir()
                .ok_or(SynchronizationError::ExternalChange)?,
        );
        self.persisted_images_match()
    }

    #[cfg(unix)]
    fn install_prepared(&self) -> Result<(), SynchronizationError> {
        self.verify_sentinel()?;
        let (_, current) = index_file_image_at(&self.parent, &fixed_index_name("index"))?;
        let Some((device, inode, digest)) = self.artifact.output else {
            return Err(SynchronizationError::RecoveryRequired);
        };
        if current.device == device
            && current.inode == inode
            && *blake3::hash(&current.bytes).as_bytes() == digest
        {
            return Ok(());
        }
        if current.bytes != self.baseline.bytes
            || (current.device, current.inode) != self.artifact.baseline_identity
        {
            return Err(SynchronizationError::ExternalChange);
        }
        let (_, prepared) = index_file_image_at(&self.staging, &fixed_index_name("index"))?;
        if prepared.device != device
            || prepared.inode != inode
            || *blake3::hash(&prepared.bytes).as_bytes() != digest
        {
            return Err(SynchronizationError::ExternalChange);
        }
        let install = fixed_index_name("install");
        if index_leaf_present(&self.staging, &install)? {
            let (_, image) = index_file_image_at(&self.staging, &install)?;
            if !index_image_is_exact(&image, &prepared) {
                return Err(SynchronizationError::ExternalChange);
            }
        } else if unsafe {
            libc::linkat(
                self.staging.as_raw_fd(),
                fixed_index_name("index").as_ptr(),
                self.staging.as_raw_fd(),
                install.as_ptr(),
                0,
            )
        } != 0
        {
            return Err(SynchronizationError::ExternalChange);
        }
        #[cfg(test)]
        tests::process_death_boundary(&self.workdir, "index-install-linked");
        if unsafe {
            libc::renameat(
                self.staging.as_raw_fd(),
                install.as_ptr(),
                self.parent.as_raw_fd(),
                fixed_index_name("index").as_ptr(),
            )
        } != 0
        {
            return Err(SynchronizationError::ExternalChange);
        }
        #[cfg(test)]
        tests::process_death_boundary(&self.workdir, "index-renamed");
        self.parent
            .sync_all()
            .and_then(|()| self.staging.sync_all())
            .map_err(|_| SynchronizationError::ExternalChange)?;
        self.persisted_images_match()
    }

    #[cfg(not(unix))]
    fn persist(
        &mut self,
        _source: &mut git2::Index,
        _repository: &git2::Repository,
        _service: &RepositoryService,
        _root: &Path,
        _owner: &RemoteReservation,
        _attempt: OperationId,
    ) -> Result<(), SynchronizationError> {
        Err(SynchronizationError::ExternalChange)
    }

    #[cfg(unix)]
    fn metadata_matches(&self, allow_absent: bool) -> Result<(), SynchronizationError> {
        self.verify_namespace()?;
        for (ordinal, member) in RESOLUTION_MERGE_MEMBERS.iter().enumerate() {
            let name = fixed_index_name(member);
            let present = index_leaf_present(&self.parent, &name)?;
            if present {
                let (_, image) = index_file_image_at(&self.parent, &name)?;
                if self.artifact.metadata[ordinal] != Some(*blake3::hash(&image.bytes).as_bytes()) {
                    return Err(SynchronizationError::ExternalChange);
                }
            } else if !allow_absent && self.artifact.metadata[ordinal].is_some() {
                return Err(SynchronizationError::ExternalChange);
            }
        }
        Ok(())
    }

    #[cfg(unix)]
    fn retire_metadata(&self) -> Result<(), SynchronizationError> {
        self.persisted_images_match()?;
        self.metadata_matches(true)?;
        // Absence is an observation of completion for each fixed member; a
        // foreign remnant stops without erasing it or releasing the sentinel.
        for member in RESOLUTION_MERGE_MEMBERS {
            let name = fixed_index_name(member);
            if index_leaf_present(&self.parent, &name)? {
                self.metadata_matches(true)?;
                if unsafe { libc::unlinkat(self.parent.as_raw_fd(), name.as_ptr(), 0) } != 0 {
                    return Err(SynchronizationError::ExternalChange);
                }
                #[cfg(test)]
                tests::process_death_boundary(
                    &self.workdir,
                    &format!("metadata-{member}-unlinked"),
                );
                self.parent
                    .sync_all()
                    .map_err(|_| SynchronizationError::ExternalChange)?;
                #[cfg(test)]
                tests::process_death_boundary(&self.workdir, &format!("metadata-{member}-barrier"));
            }
        }
        Ok(())
    }

    #[cfg(unix)]
    fn retire(
        &mut self,
        service: &RepositoryService,
        root: &Path,
        owner: &RemoteReservation,
        attempt: OperationId,
    ) -> Result<(), SynchronizationError> {
        if self.artifact.phase == "published" {
            self.persisted_images_match()?;
            service.advance_synchronization_index_artifact(
                root,
                owner,
                attempt,
                "release_intent",
            )?;
            self.artifact.phase = "release_intent".into();
        }
        #[cfg(test)]
        run_resolution_index_retire_hook(&self.workdir);
        self.verify_namespace()?;
        if index_leaf_present(&self.parent, &fixed_index_name("index.lock"))? {
            if self.artifact.phase == "released" {
                return Err(SynchronizationError::ExternalChange);
            }
            self.verify_sentinel()?;
            // Verified unlink under the approved cooperative namespace convention,
            // not an expected-inode CAS. The retained anchor survives release.
            if unsafe {
                libc::unlinkat(
                    self.parent.as_raw_fd(),
                    fixed_index_name("index.lock").as_ptr(),
                    0,
                )
            } != 0
            {
                return Err(SynchronizationError::ExternalChange);
            }
            #[cfg(test)]
            tests::process_death_boundary(&self.workdir, "sentinel-unlinked");
        }
        self.parent
            .sync_all()
            .map_err(|_| SynchronizationError::ExternalChange)?;
        #[cfg(test)]
        tests::process_death_boundary(&self.workdir, "sentinel-release-barrier");
        service.advance_synchronization_index_artifact(root, owner, attempt, "released")?;
        self.artifact.phase = "released".into();
        Ok(())
    }

    #[cfg(not(unix))]
    fn retire_metadata(&self) -> Result<(), SynchronizationError> {
        Err(SynchronizationError::ExternalChange)
    }
    #[cfg(not(unix))]
    fn retire(
        &mut self,
        _service: &RepositoryService,
        _root: &Path,
        _owner: &RemoteReservation,
        _attempt: OperationId,
    ) -> Result<(), SynchronizationError> {
        Err(SynchronizationError::ExternalChange)
    }
}

fn preflight_bytes(hasher: &mut blake3::Hasher, bytes: &[u8]) {
    hasher.update(&(bytes.len() as u64).to_be_bytes());
    hasher.update(bytes);
}

fn preflight_path_bytes(path: &Path) -> Vec<u8> {
    path.to_string_lossy().as_bytes().to_vec()
}

fn conflict_worktree_digest(
    root: &Path,
    relative: &Path,
) -> Result<[u8; 32], SynchronizationError> {
    let mut digest = blake3::Hasher::new();
    digest.update(b"manyhands-resolution-prewrite-v1\0");
    match owned_resolution_file_bytes(
        root,
        relative,
        RepositoryOperation::RepositorySnapshot,
        root,
    ) {
        Ok(Some(bytes)) => {
            digest.update(&[1]);
            preflight_bytes(&mut digest, &bytes);
        }
        Ok(None) => {
            digest.update(&[0]);
        }
        Err(_) => return Err(SynchronizationError::ExternalChange),
    }
    Ok(*digest.finalize().as_bytes())
}

fn resolution_path_needs_write(
    root: &Path,
    token: &merge::ConflictPathToken,
    result: &[u8],
    durable: &state::ResolutionPathEvidence,
) -> Result<bool, SynchronizationError> {
    if durable.ordinal != token.ordinal || *blake3::hash(result).as_bytes() != durable.result_digest
    {
        return Err(SynchronizationError::ExternalChange);
    }
    let relative = Path::new(
        std::str::from_utf8(&token.path).map_err(|_| SynchronizationError::ExternalChange)?,
    );
    let bytes = owned_resolution_file_bytes(
        root,
        relative,
        RepositoryOperation::RepositorySnapshot,
        root,
    )
    .map_err(|_| SynchronizationError::ExternalChange)?;
    if bytes
        .as_deref()
        .map(|bytes| *blake3::hash(bytes).as_bytes())
        == Some(durable.result_digest)
    {
        return Ok(false);
    }
    if !durable.applied {
        let mut prewrite = blake3::Hasher::new();
        prewrite.update(b"manyhands-resolution-prewrite-v1\0");
        match bytes {
            Some(bytes) => {
                prewrite.update(&[1]);
                preflight_bytes(&mut prewrite, &bytes);
            }
            None => {
                prewrite.update(&[0]);
            }
        }
        if *prewrite.finalize().as_bytes() == durable.prewrite_digest {
            return Ok(true);
        }
    }
    Err(SynchronizationError::ExternalChange)
}

fn owned_result_digest(root: &Path, relative: &Path) -> Result<[u8; 32], SynchronizationError> {
    let bytes = owned_resolution_file_bytes(
        root,
        relative,
        RepositoryOperation::RepositorySnapshot,
        root,
    )
    .map_err(|_| SynchronizationError::ExternalChange)?
    .ok_or(SynchronizationError::ExternalChange)?;
    Ok(*blake3::hash(&bytes).as_bytes())
}

fn conflict_token_digest(token: &merge::ConflictPathToken) -> [u8; 32] {
    let mut expected = blake3::Hasher::new();
    expected.update(b"manyhands-conflict-path-v1\0");
    // Keep existing window-zero attempt digests byte-for-byte compatible.
    // New passes bind capabilities to their immutable window and stage.
    if token.observation.window_number != 0 {
        expected.update(b"window\0");
        expected.update(&token.observation.window_number.to_be_bytes());
        expected.update(&[token.observation.ordinal]);
    }
    for (oid, mode) in [
        (token.base, token.base_mode),
        (token.local, token.local_mode),
        (token.incoming, token.incoming_mode),
    ] {
        expected.update(
            oid.map(|value| value.as_bytes().to_vec())
                .as_deref()
                .unwrap_or(&[]),
        );
        expected.update(&mode.unwrap_or_default().to_be_bytes());
    }
    *expected.finalize().as_bytes()
}

/// Fingerprint every local input which an owned resolution must not silently
/// absorb. Git's conflict fingerprint intentionally omits ordinary index and
/// worktree state, so it cannot be used for this purpose.
fn resolution_preflight(
    repository: &git2::Repository,
    resolutions: &[(merge::ConflictPathToken, merge::RedactedConflictBytes)],
) -> Result<[u8; 32], SynchronizationError> {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"manyhands-resolution-preflight-v1\0");

    let index = repository
        .index()
        .map_err(|_| SynchronizationError::RecoveryRequired)?;
    let mut entries = index
        .iter()
        .map(|entry| {
            (
                entry.path.clone(),
                (entry.flags >> 12) & 3,
                entry.mode,
                entry.id,
            )
        })
        .collect::<Vec<_>>();
    entries.sort_by(|left, right| left.0.cmp(&right.0).then(left.1.cmp(&right.1)));
    hasher.update(&(entries.len() as u64).to_be_bytes());
    for (path, stage, mode, oid) in entries {
        preflight_bytes(&mut hasher, &path);
        hasher.update(&stage.to_be_bytes());
        hasher.update(&mode.to_be_bytes());
        hasher.update(oid.as_bytes());
    }

    let mut options = resolution_status_options();
    let statuses = repository
        .statuses(Some(&mut options))
        .map_err(|_| SynchronizationError::RecoveryRequired)?;
    let mut status_entries = statuses
        .iter()
        .filter(|entry| !entry.status().is_ignored())
        .map(|entry| {
            let mut paths = Vec::new();
            if let Some(path) = entry.path() {
                paths.push(path.as_bytes().to_vec());
            }
            for delta in [entry.head_to_index(), entry.index_to_workdir()]
                .into_iter()
                .flatten()
            {
                for file in [delta.old_file(), delta.new_file()] {
                    if let Some(path) = file.path() {
                        paths.push(preflight_path_bytes(path));
                    }
                }
            }
            paths.sort();
            (entry.status().bits(), paths)
        })
        .collect::<Vec<_>>();
    status_entries.sort();
    hasher.update(&(status_entries.len() as u64).to_be_bytes());
    for (status, paths) in status_entries {
        hasher.update(&status.to_be_bytes());
        hasher.update(&(paths.len() as u64).to_be_bytes());
        for path in paths {
            preflight_bytes(&mut hasher, &path);
        }
    }

    let workdir = repository
        .workdir()
        .ok_or(SynchronizationError::RecoveryRequired)?;
    let mut paths = resolutions.iter().collect::<Vec<_>>();
    paths.sort_by_key(|(token, _)| token.ordinal);
    if paths
        .windows(2)
        .any(|pair| pair[0].0.ordinal == pair[1].0.ordinal)
    {
        return Err(SynchronizationError::RecoveryRequired);
    }
    hasher.update(&(paths.len() as u64).to_be_bytes());
    for (token, _) in paths {
        let path_text =
            std::str::from_utf8(&token.path).map_err(|_| SynchronizationError::ExternalChange)?;
        let path = Path::new(path_text);
        if path.as_os_str().is_empty()
            || !path
                .components()
                .all(|component| matches!(component, std::path::Component::Normal(_)))
        {
            return Err(SynchronizationError::ExternalChange);
        }
        preflight_bytes(&mut hasher, &token.ordinal.to_be_bytes());
        preflight_bytes(&mut hasher, &token.path);
        // One descriptor-based O_NOFOLLOW open both verifies the leaf and
        // reads it. Do not split this into metadata followed by fs::read:
        // another process could replace the leaf with a symlink in between.
        match owned_resolution_file_bytes(
            workdir,
            path,
            RepositoryOperation::RepositorySnapshot,
            workdir,
        ) {
            Ok(Some(bytes)) => {
                hasher.update(&[1]);
                preflight_bytes(&mut hasher, &bytes);
            }
            Ok(None) => {
                hasher.update(&[0]);
            }
            Err(_) => return Err(SynchronizationError::ExternalChange),
        }
    }
    Ok(*hasher.finalize().as_bytes())
}

/// The non-conflicting stage-zero portion of an owned merge is wholly
/// determined by its recorded parents. Never accept a caller's unrelated
/// staged entry merely because the conflict paths themselves still match.
/// One status policy for initial preflight, final validation, and candidate
/// recovery. Ignored entries are requested so the policy is explicit, then
/// excluded from mutable-input evidence: ignored files are deliberately
/// permitted and must not make an exact post-ref retry non-idempotent.
fn resolution_status_options() -> git2::StatusOptions {
    let mut options = git2::StatusOptions::new();
    options
        .include_untracked(true)
        .recurse_untracked_dirs(true)
        .include_ignored(true)
        .recurse_ignored_dirs(true)
        .renames_head_to_index(true)
        .renames_index_to_workdir(true);
    options
}

fn only_resolution_paths_are_dirty(
    repository: &git2::Repository,
    resolutions: &[(merge::ConflictPathToken, merge::RedactedConflictBytes)],
) -> Result<bool, SynchronizationError> {
    let allowed = resolutions
        .iter()
        .map(|(token, _)| token.path.clone())
        .collect::<std::collections::BTreeSet<_>>();
    only_paths_are_dirty(repository, &allowed)
}

fn only_paths_are_dirty(
    repository: &git2::Repository,
    allowed: &std::collections::BTreeSet<Vec<u8>>,
) -> Result<bool, SynchronizationError> {
    let mut options = resolution_status_options();
    for entry in repository
        .statuses(Some(&mut options))
        .map_err(|_| SynchronizationError::RecoveryRequired)?
        .iter()
    {
        if entry.status().is_ignored() {
            continue;
        }
        // HEAD -> index changes include Git's clean incoming merge entries.
        // The authoritative merge-index comparison fences unrelated staging;
        // this gate fences only worktree/untracked changes outside the tokens.
        if !entry.status().intersects(
            git2::Status::WT_NEW
                | git2::Status::WT_MODIFIED
                | git2::Status::WT_DELETED
                | git2::Status::WT_TYPECHANGE
                | git2::Status::WT_RENAMED
                | git2::Status::CONFLICTED,
        ) {
            continue;
        }
        let mut paths = Vec::new();
        if let Some(path) = entry.path() {
            paths.push(path.as_bytes().to_vec());
        }
        if let Some(delta) = entry.index_to_workdir() {
            for file in [delta.old_file(), delta.new_file()] {
                if let Some(path) = file.path() {
                    paths.push(preflight_path_bytes(path));
                }
            }
        }
        if paths.is_empty() || paths.iter().any(|path| !allowed.contains(path)) {
            return Ok(false);
        }
    }
    Ok(true)
}

fn recorded_merge_index_matches_index(
    repository: &git2::Repository,
    actual: &git2::Index,
    local: git2::Oid,
    incoming: git2::Oid,
) -> Result<bool, SynchronizationError> {
    let expected = prepare_recorded_merge_index(repository, local, incoming)?;
    prepared_merge_index_matches_index(&expected, actual)
}

/// libgit2 merge_commits can generate clean blobs even when another path
/// conflicts. Inspection must route those writes into a private overlay on a
/// separately opened handle, never the destination's normal ODB. The returned
/// index owns only stage entries/OIDs; no generated bytes are imported.
fn prepare_recorded_merge_index(
    repository: &git2::Repository,
    local: git2::Oid,
    incoming: git2::Oid,
) -> Result<git2::Index, SynchronizationError> {
    let worker = git2::Repository::open(repository.path())
        .map_err(|_| SynchronizationError::RecoveryRequired)?;
    let odb = worker
        .odb()
        .map_err(|_| SynchronizationError::RecoveryRequired)?;
    let _mempack = odb
        .add_new_mempack_backend(1000)
        .map_err(|_| SynchronizationError::RecoveryRequired)?;
    let local = worker
        .find_commit(local)
        .map_err(|_| SynchronizationError::RecoveryRequired)?;
    let incoming = worker
        .find_commit(incoming)
        .map_err(|_| SynchronizationError::RecoveryRequired)?;
    worker
        .merge_commits(&local, &incoming, None)
        .map_err(|_| SynchronizationError::RecoveryRequired)
}

fn prepared_merge_index_matches_index(
    expected: &git2::Index,
    actual: &git2::Index,
) -> Result<bool, SynchronizationError> {
    let conflicts = expected
        .conflicts()
        .map_err(|_| SynchronizationError::RecoveryRequired)?
        .map(|conflict| {
            let conflict = conflict.map_err(|_| SynchronizationError::RecoveryRequired)?;
            conflict
                .our
                .or(conflict.their)
                .or(conflict.ancestor)
                .map(|entry| entry.path)
                .ok_or(SynchronizationError::RecoveryRequired)
        })
        .collect::<Result<std::collections::BTreeSet<_>, _>>()?;
    let stage = |entry: &git2::IndexEntry| (entry.flags >> 12) & 3;
    let entries = |index: &git2::Index| {
        index
            .iter()
            .filter(|entry| stage(entry) == 0 && !conflicts.contains(&entry.path))
            .map(|entry| (entry.path.clone(), entry.mode, entry.id))
            .collect::<Vec<_>>()
    };
    let mut expected_entries = entries(expected);
    let mut actual_entries = entries(actual);
    expected_entries.sort();
    actual_entries.sort();
    Ok(expected_entries == actual_entries)
}

fn recorded_merge_index_matches(
    repository: &git2::Repository,
    local: git2::Oid,
    incoming: git2::Oid,
) -> Result<bool, SynchronizationError> {
    let actual = repository
        .index()
        .map_err(|_| SynchronizationError::RecoveryRequired)?;
    recorded_merge_index_matches_index(repository, &actual, local, incoming)
}

fn foreign_resolution_metadata_present(repository: &git2::Repository) -> bool {
    [
        "REBASE_HEAD",
        "CHERRY_PICK_HEAD",
        "REVERT_HEAD",
        "BISECT_LOG",
        "rebase-merge",
        "rebase-apply",
        "sequencer",
    ]
    .iter()
    .any(|name| {
        [repository.path(), repository.commondir()]
            .into_iter()
            .any(|root| match std::fs::symlink_metadata(root.join(name)) {
                Ok(_) => true,
                Err(error) => error.kind() != std::io::ErrorKind::NotFound,
            })
    })
}

/// Only the exact merge that this integration step recorded may be retired.
/// Foreign rebase/cherry-pick state is never cleanup authority.
fn exact_resolution_merge_metadata(
    repository: &mut git2::Repository,
    incoming: git2::Oid,
) -> Result<bool, SynchronizationError> {
    if foreign_resolution_metadata_present(repository) {
        return Ok(false);
    }
    let mut heads = Vec::new();
    if repository
        .mergehead_foreach(|oid| {
            heads.push(*oid);
            true
        })
        .is_err()
    {
        return Ok(false);
    }
    Ok(heads == [incoming])
}

fn conflict_digest(repository: &mut git2::Repository) -> Result<[u8; 32], SynchronizationError> {
    let index = repository
        .index()
        .map_err(|_| SynchronizationError::RecoveryRequired)?;
    if !index.has_conflicts() {
        return Err(SynchronizationError::RecoveryRequired);
    }
    let mut digest = conflict_index_hasher(&index)?;
    repository
        .mergehead_foreach(|oid| {
            digest.update(oid.as_bytes());
            true
        })
        .map_err(|_| SynchronizationError::RecoveryRequired)?;
    Ok(*digest.finalize().as_bytes())
}

fn conflict_index_hasher(index: &git2::Index) -> Result<blake3::Hasher, SynchronizationError> {
    let mut digest = blake3::Hasher::new();
    for conflict in index
        .conflicts()
        .map_err(|_| SynchronizationError::RecoveryRequired)?
    {
        let conflict = conflict.map_err(|_| SynchronizationError::RecoveryRequired)?;
        for entry in [conflict.ancestor, conflict.our, conflict.their]
            .into_iter()
            .flatten()
        {
            digest.update(&entry.mode.to_le_bytes());
            digest.update(entry.id.as_bytes());
            digest.update(&entry.path);
        }
    }
    Ok(digest)
}

fn bind_conflict_digest(raw: [u8; 32], window_number: u32, ordinal: u8) -> [u8; 32] {
    if window_number == 0 {
        return raw;
    }
    let mut digest = blake3::Hasher::new();
    digest.update(b"manyhands-window-conflict-v1\0");
    digest.update(&window_number.to_be_bytes());
    digest.update(&[ordinal]);
    digest.update(&raw);
    *digest.finalize().as_bytes()
}

fn integration_conflict_digest(
    repository: &mut git2::Repository,
    window_number: u32,
    ordinal: u8,
) -> Result<[u8; 32], SynchronizationError> {
    Ok(bind_conflict_digest(
        conflict_digest(repository)?,
        window_number,
        ordinal,
    ))
}

fn prepared_conflict_digest(
    expected: &git2::Index,
    step: &state::IntegrationStepEvidence,
) -> Result<[u8; 32], SynchronizationError> {
    if !expected.has_conflicts() {
        return Err(SynchronizationError::RecoveryRequired);
    }
    let mut digest = conflict_index_hasher(expected)?;
    digest.update(step.intent.incoming_oid.as_bytes());
    Ok(bind_conflict_digest(
        *digest.finalize().as_bytes(),
        step.window_number,
        step.intent.ordinal,
    ))
}

fn refuse_local_reconciliation_locks(
    repository: &git2::Repository,
) -> Result<(), SynchronizationError> {
    // A generic integration has no owned index sentinel/ref-lock manifest.
    // Presence or uncertain inspection requires human verification, never
    // lock deletion, PID/age inference or automatic ref/log replay.
    match std::fs::symlink_metadata(repository.path().join("index.lock")) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        _ => return Err(SynchronizationError::RecoveryRequired),
    }
    #[cfg(any(unix, windows))]
    refuse_ambiguous_resolution_backend_locks(repository)?;
    Ok(())
}

fn configuration_identity_digest(
    repository: &git2::Repository,
) -> Result<[u8; 32], SynchronizationError> {
    let config = repository
        .config()
        .map_err(|_| SynchronizationError::RecoveryRequired)?;
    let mut digest = blake3::Hasher::new();
    for key in ["user.name", "user.email"] {
        digest.update(key.as_bytes());
        if let Ok(value) = config.get_string(key) {
            digest.update(value.as_bytes());
        }
        digest.update(&[0]);
    }
    Ok(*digest.finalize().as_bytes())
}

fn effective_identity(
    repository: &git2::Repository,
) -> Result<Option<CommitIdentity>, SynchronizationError> {
    let config = repository
        .config()
        .map_err(|_| SynchronizationError::RecoveryRequired)?;
    let name = config.get_string("user.name").ok();
    let email = config.get_string("user.email").ok();
    Ok(match (name, email) {
        (Some(name), Some(email)) if !name.is_empty() && !email.is_empty() => {
            Some(CommitIdentity { name, email })
        }
        _ => None,
    })
}

fn committing_identity(
    service: &RepositoryService,
    root: &Path,
    owner: &RemoteReservation,
    repository: &git2::Repository,
    request: &SynchronizeRemoteRequest,
) -> Result<CommitIdentity, SynchronizationError> {
    if let Some(identity) = effective_identity(repository)? {
        return Ok(identity);
    }
    let confirmation = request.confirmed_identity.as_ref().ok_or_else(|| {
        SynchronizationError::IdentityRequired {
            target: request.target.clone(),
        }
    })?;
    if confirmation.identity.name.is_empty()
        || confirmation.identity.email.is_empty()
        || confirmation.identity.name.contains('\0')
        || confirmation.identity.email.contains('\0')
        || configuration_identity_digest(repository)? != confirmation.expected_configuration
    {
        return Err(SynchronizationError::ExternalChange);
    }
    let mut input = blake3::Hasher::new();
    input.update(confirmation.confirmation_id.to_string().as_bytes());
    input.update(confirmation.identity.name.as_bytes());
    input.update(&[0]);
    input.update(confirmation.identity.email.as_bytes());
    service.prepare_synchronization_identity_confirmation(
        root,
        owner,
        &state::IdentityConfirmationIntent {
            confirmation_id: confirmation.confirmation_id,
            input_digest: *input.finalize().as_bytes(),
            configuration_digest: confirmation.expected_configuration,
        },
    )?;
    service.begin_synchronization_identity_confirmation_effect(
        root,
        owner,
        confirmation.confirmation_id,
    )?;
    let mut config = repository
        .config()
        .map_err(|_| SynchronizationError::RecoveryRequired)?;
    config
        .set_str("user.name", &confirmation.identity.name)
        .and_then(|_| config.set_str("user.email", &confirmation.identity.email))
        .map_err(|_| SynchronizationError::RecoveryRequired)?;
    let observed = configuration_identity_digest(repository)?;
    service.observe_synchronization_identity_confirmation_effect(
        root,
        owner,
        confirmation.confirmation_id,
        observed,
    )?;
    Ok(confirmation.identity.clone())
}

#[derive(Clone, Copy)]
struct ReconciledCandidate {
    window_number: u32,
    ordinal: u8,
    oid: git2::Oid,
    tree: git2::Oid,
}

/// Reconcile the newest recorded local stage before any clean-target preflight
/// or network. Completed effects are observations, never new merge preparation.
/// Publication still requires fresh direction-specific remote evidence.
fn reconcile_pending_candidate(
    service: &RepositoryService,
    root: &Path,
    primary_branch: &str,
    target: &SynchronizationTarget,
    owner: &RemoteReservation,
    evidence: &mut state::SynchronizationEvidence,
) -> Result<Option<ReconciledCandidate>, SynchronizationError> {
    let Some(step) = service.applying_synchronization_candidate(root, owner)? else {
        return Ok(None);
    };
    service.synchronization_boundary(root, owner)?;
    let repository = materialized_target(root, primary_branch, target)
        .map_err(|_| SynchronizationError::RecoveryRequired)?;
    refuse_local_reconciliation_locks(&repository)?;
    let unfinished_conflict = step.phase == state::IntegrationStepPhase::Applying
        && step.candidate_oid.is_none()
        && repository
            .index()
            .map_err(|_| SynchronizationError::RecoveryRequired)?
            .has_conflicts();
    if step.phase == state::IntegrationStepPhase::ConflictPending || unfinished_conflict {
        let head = local_oid(&repository)?;
        if head == step.intent.local_oid {
            // Prepare one isolated snapshot for both conflict-stage and
            // non-conflicting stage-zero checks. Rejection stays ODB-read-only
            // even before current merge metadata is proved under the lease.
            let expected = prepare_recorded_merge_index(
                &repository,
                step.intent.local_oid,
                step.intent.incoming_oid,
            )?;
            let expected_conflict = if unfinished_conflict {
                Some(prepared_conflict_digest(&expected, &step)?)
            } else {
                step.conflict_digest
            };
            let index = repository
                .index()
                .map_err(|_| SynchronizationError::RecoveryRequired)?;
            let allowed = index
                .iter()
                .filter(|entry| entry.flags >> 12 & 3 != 0)
                .map(|entry| entry.path)
                .collect::<std::collections::BTreeSet<_>>();
            if !only_paths_are_dirty(&repository, &allowed)? {
                return Err(SynchronizationError::RecoveryRequired);
            }
            let baseline_matches = prepared_merge_index_matches_index(&expected, &index)?;
            let preflight = resolution_preflight(&repository, &[])?;
            #[cfg(test)]
            run_resolution_index_hook(&LOCAL_RECONCILIATION_PREPARED_HOOK, root);
            let mut fresh = materialized_target(root, primary_branch, target)?;
            let _lease = repository_lease(&fresh, root, RepositoryOperation::RepositorySnapshot)?;
            service.synchronization_boundary(root, owner)?;
            refuse_local_reconciliation_locks(&fresh)?;
            // The preflight fingerprint covers index and status, not HEAD. A
            // same-tree ref movement during isolated preparation must not let
            // this conflict be recorded against the frozen local parent.
            if local_oid(&fresh)? != step.intent.local_oid
                || !exact_resolution_merge_metadata(&mut fresh, step.intent.incoming_oid)?
                || integration_conflict_digest(&mut fresh, step.window_number, step.intent.ordinal)
                    .ok()
                    != expected_conflict
                || !baseline_matches
                || resolution_preflight(&fresh, &[])? != preflight
            {
                return Err(SynchronizationError::RecoveryRequired);
            }
            if unfinished_conflict {
                service.release_synchronization_conflict_in_window(
                    root,
                    owner,
                    step.window_number,
                    step.intent.ordinal,
                    expected_conflict.ok_or(SynchronizationError::RecoveryRequired)?,
                )?;
            } else {
                service.release_inspected_synchronization_conflict(root, owner, &step)?;
            }
            return Err(SynchronizationError::ConflictPending {
                target: target.clone(),
                operation_id: owner.operation_id(),
                stage: stage_name(step.intent.stage),
            });
        }
        // Full context validation is deliberately outside the common Git lease.
        // Staged resolutions or marker removal alone are never completion proof.
        require_clean_target(&repository, target)
            .map_err(|_| SynchronizationError::RecoveryRequired)?;
        let commit = repository
            .find_commit(head)
            .map_err(|_| SynchronizationError::RecoveryRequired)?;
        if commit.parent_count() != 2
            || [commit.parent_id(0).ok(), commit.parent_id(1).ok()]
                != [Some(step.intent.local_oid), Some(step.intent.incoming_oid)]
            || !RepositoryService::validates_external_integration(&repository, &step, head)?
        {
            return Err(SynchronizationError::RecoveryRequired);
        }
        let tree = commit.tree_id();
        let _lease = repository_lease(&repository, root, RepositoryOperation::RepositorySnapshot)?;
        service.synchronization_boundary(root, owner)?;
        let fresh = materialized_target(root, primary_branch, target)
            .map_err(|_| SynchronizationError::RecoveryRequired)?;
        refuse_local_reconciliation_locks(&fresh)?;
        require_clean_target(&fresh, target).map_err(|_| SynchronizationError::RecoveryRequired)?;
        if local_oid(&fresh)? != head {
            return Err(SynchronizationError::RecoveryRequired);
        }
        service.observe_external_synchronization_integration(root, owner, &step, head, tree)?;
        evidence.local_oid = Some(head);
        return Ok(Some(ReconciledCandidate {
            window_number: step.window_number,
            ordinal: step.intent.ordinal,
            oid: head,
            tree,
        }));
    }
    if matches!(
        step.phase,
        state::IntegrationStepPhase::ResolutionPrepared
            | state::IntegrationStepPhase::CommitPrepared
    ) {
        // Only the identical resolution request can resume its native artifacts.
        return Err(SynchronizationError::RecoveryRequired);
    }
    if step.phase == state::IntegrationStepPhase::Prepared {
        let fresh = local_target(root, primary_branch, target)
            .map_err(|_| SynchronizationError::RecoveryRequired)?;
        if local_oid(&fresh)? != step.intent.local_oid {
            return Err(SynchronizationError::RecoveryRequired);
        }
        // There is no proven effect to observe. Its frozen intent is resumed
        // locally by ordered integration, never rebound to newly fetched refs.
        return Ok(None);
    }
    let candidate = if let Some(candidate) = step.candidate_oid.or(step.result_oid) {
        candidate
    } else {
        match merge::classify_integration(
            step.intent.local_oid,
            step.intent.incoming_oid,
            |older, newer| repository.graph_descendant_of(newer, older),
            |left, right| repository.merge_base(left, right).map(|_| true),
        )
        .map_err(|_| SynchronizationError::RecoveryRequired)?
        {
            merge::IntegrationDisposition::Equal
            | merge::IntegrationDisposition::IncomingAlreadyIntegrated => step.intent.local_oid,
            merge::IntegrationDisposition::FastForward => step.intent.incoming_oid,
            merge::IntegrationDisposition::MergeRequired => {
                return Err(SynchronizationError::RecoveryRequired);
            }
        }
    };
    require_clean_target(&repository, target)
        .map_err(|_| SynchronizationError::RecoveryRequired)?;
    let observed_head = local_oid(&repository)?;
    if step.phase == state::IntegrationStepPhase::Applied && observed_head != candidate {
        let released = state::with_transaction(service, root, |tx, id| {
            let record = reservation::owned(service, tx, id, owner)?;
            state::integration_resolution_released(tx, &record, &step)
        })?;
        if !released
            || step.intent.stage != merge::IntegrationStage::Primary
            || step.candidate_oid != Some(candidate)
            || step.result_oid != Some(candidate)
            || !repository
                .graph_descendant_of(observed_head, candidate)
                .unwrap_or(false)
            || !RepositoryService::validates_external_integration(
                &repository,
                &step,
                observed_head,
            )?
        {
            return Err(SynchronizationError::RecoveryRequired);
        }
        // A completed and released local checkpoint is observation-only. A
        // later clean descendant is new pass input, never a changed candidate
        // or permission to replay the old transition/ref-log protocol.
        let tree = repository
            .find_commit(candidate)
            .map_err(|_| SynchronizationError::RecoveryRequired)?
            .tree_id();
        if step.observed_tree_oid != Some(tree) {
            return Err(SynchronizationError::RecoveryRequired);
        }
        let _lease = repository_lease(&repository, root, RepositoryOperation::RepositorySnapshot)?;
        service.synchronization_boundary(root, owner)?;
        let fresh = materialized_target(root, primary_branch, target)?;
        refuse_local_reconciliation_locks(&fresh)?;
        require_clean_target(&fresh, target).map_err(|_| SynchronizationError::RecoveryRequired)?;
        if local_oid(&fresh)? != observed_head {
            return Err(SynchronizationError::RecoveryRequired);
        }
        evidence.local_oid = Some(observed_head);
        return Ok(Some(ReconciledCandidate {
            window_number: step.window_number,
            ordinal: step.intent.ordinal,
            oid: candidate,
            tree,
        }));
    }
    let _lease = repository_lease(&repository, root, RepositoryOperation::RepositorySnapshot)?;
    service.synchronization_boundary(root, owner)?;
    let fresh = materialized_target(root, primary_branch, target)
        .map_err(|_| SynchronizationError::RecoveryRequired)?;
    refuse_local_reconciliation_locks(&fresh)?;
    require_clean_target(&fresh, target).map_err(|_| SynchronizationError::RecoveryRequired)?;
    let head = local_oid(&fresh)?;
    let commit = repository
        .find_commit(candidate)
        .map_err(|_| SynchronizationError::RecoveryRequired)?;
    let parents = [commit.parent_id(0).ok(), commit.parent_id(1).ok()];
    if step.candidate_oid.is_some()
        && (commit.parent_count() != 2
            || parents != [Some(step.intent.local_oid), Some(step.intent.incoming_oid)])
    {
        return Err(SynchronizationError::RecoveryRequired);
    }
    let tree = commit.tree_id();
    if step.phase == state::IntegrationStepPhase::Applied {
        // A released resolution has already proved and completed its native
        // ref/log effect. Restart only observes that exact checkpoint; external
        // restoration of an old/third HEAD never authorizes repeating it via
        // the generic fast-forward path.
        if head != candidate
            || step.result_oid != Some(candidate)
            || step.observed_tree_oid != Some(tree)
        {
            return Err(SynchronizationError::RecoveryRequired);
        }
    } else if step.phase != state::IntegrationStepPhase::Applying {
        return Err(SynchronizationError::RecoveryRequired);
    } else if head == step.intent.local_oid {
        let old = repository
            .find_commit(head)
            .map_err(|_| SynchronizationError::RecoveryRequired)?;
        if old.tree_id() != step.intent.baseline_tree_oid
            || index_digest(old.tree_id()) != step.intent.baseline_index_digest
        {
            return Err(SynchronizationError::RecoveryRequired);
        }
        let branch = repository
            .find_reference("HEAD")
            .ok()
            .and_then(|reference| reference.symbolic_target().map(str::to_owned))
            .ok_or(SynchronizationError::RecoveryRequired)?;
        if head != candidate {
            service.synchronization_boundary(root, owner)?;
            fast_forward(&repository, &branch, head, candidate)?;
        }
    } else if head != candidate {
        return Err(SynchronizationError::RecoveryRequired);
    }
    let observed = local_oid(&repository)?;
    if observed != candidate {
        return Err(SynchronizationError::RecoveryRequired);
    }
    if step.phase == state::IntegrationStepPhase::Applying {
        service.observe_synchronization_integration_effect_in_window(
            root,
            owner,
            step.window_number,
            step.intent.ordinal,
            candidate,
            tree,
        )?;
    }
    evidence.local_oid = Some(candidate);
    Ok(Some(ReconciledCandidate {
        window_number: step.window_number,
        ordinal: step.intent.ordinal,
        oid: candidate,
        tree,
    }))
}

/// Once the restarted Fetch has replaced the frozen tracking evidence, commit
/// the already-observed candidate into the outer synchronization envelope.
fn finalize_reconciled_candidate(
    service: &RepositoryService,
    root: &Path,
    owner: &RemoteReservation,
    candidate: ReconciledCandidate,
    evidence: &state::SynchronizationEvidence,
) -> Result<(), SynchronizationError> {
    decision(
        service.reconcile_synchronization_candidate_applied_in_window(
            root,
            owner,
            candidate.window_number,
            candidate.ordinal,
            candidate.oid,
            candidate.tree,
            evidence,
        )?,
    )
}

#[derive(Clone, Copy)]
struct DivergenceInputs<'a> {
    root: &'a Path,
    primary_branch: &'a str,
    target: &'a SynchronizationTarget,
    request: &'a SynchronizeRemoteRequest,
    owner: &'a RemoteReservation,
    plan: &'a RemoteRefPlan,
    configuration: &'a super::observation::ObservationConfiguration,
    selected: &'a RemoteRefTarget,
    primary_tracking: Option<git2::Oid>,
    selected_tracking: Option<git2::Oid>,
    context: Option<git2::Oid>,
    primary: git2::Oid,
}

/// Re-observe every input that was frozen after Fetch before mutating the
/// destination. Preparation is intentionally outside the lease; it is never
/// authority to apply a merge after configuration, tracking, branch, index or
/// worktree state changes.
fn recheck_divergence_inputs(
    service: &RepositoryService,
    input: &DivergenceInputs<'_>,
    expected_local: git2::Oid,
) -> Result<(), SynchronizationError> {
    let target_repository = materialized_target(input.root, input.primary_branch, input.target)?;
    require_clean_target(&target_repository, input.target)?;
    if local_oid(&target_repository)? != expected_local
        || service.observation_configuration(input.root, input.plan)? != *input.configuration
        || target_repository
            .refname_to_id(input.plan.primary().tracking_ref())
            .ok()
            != input.primary_tracking
        || target_repository
            .refname_to_id(input.selected.tracking_ref())
            .ok()
            != input.selected_tracking
    {
        return Err(SynchronizationError::ExternalChange);
    }
    Ok(())
}

fn integrate_divergence(
    service: &RepositoryService,
    input: DivergenceInputs<'_>,
) -> Result<git2::Oid, SynchronizationError> {
    let DivergenceInputs {
        root,
        primary_branch,
        target,
        request,
        owner,
        context,
        primary,
        ..
    } = input;
    let target_repository = local_target(root, primary_branch, target)?;
    let path = target_repository
        .workdir()
        .ok_or(SynchronizationError::RecoveryRequired)?
        .to_owned();
    let mut local = local_oid(&target_repository)?;
    let window = state::with_transaction(service, root, |tx, id| {
        let record = reservation::owned(service, tx, id, owner)?;
        state::latest_integration_window(tx, &record)
    })?;
    if window
        .intent
        .as_ref()
        .is_some_and(|pass| pass.primary_oid != primary || pass.context_oid != context)
    {
        return Err(SynchronizationError::RecoveryRequired);
    }
    let stages = merge::integration_stages(target, context.is_some());
    for stage in stages {
        let ordinal = match (target, stage) {
            (SynchronizationTarget::Context { .. }, merge::IntegrationStage::Primary) => 1,
            _ => 0,
        };
        service.synchronization_boundary(root, owner)?;
        let incoming = match stage {
            merge::IntegrationStage::Context => {
                context.ok_or(SynchronizationError::RecoveryRequired)?
            }
            merge::IntegrationStage::Primary => primary,
        };
        let recorded = state::with_transaction(service, root, |tx, id| {
            let record = reservation::owned(service, tx, id, owner)?;
            state::integration_step_in_window(tx, record.id, window.number, ordinal)
        })?;
        if let Some(step) = &recorded {
            if step.intent.stage != stage || step.intent.incoming_oid != incoming {
                return Err(SynchronizationError::RecoveryRequired);
            }
            if step.phase == state::IntegrationStepPhase::Applied {
                // Earlier stages remain immutable audit evidence. HEAD is at
                // the last completed stage, not necessarily this earlier one.
                continue;
            }
            if step.phase != state::IntegrationStepPhase::Prepared || step.intent.local_oid != local
            {
                return Err(SynchronizationError::RecoveryRequired);
            }
        }
        let classification = merge::classify_integration(
            local,
            incoming,
            |older, newer| target_repository.graph_descendant_of(newer, older),
            |left, right| {
                target_repository
                    .merge_base(left, right)
                    .map(|_| true)
                    .or_else(|error| {
                        if error.code() == git2::ErrorCode::NotFound {
                            Ok(false)
                        } else {
                            Err(error)
                        }
                    })
            },
        )
        .map_err(|_| SynchronizationError::RecoveryRequired)?;
        let local_commit = target_repository
            .find_commit(local)
            .map_err(|_| SynchronizationError::RecoveryRequired)?;
        let intent = state::IntegrationStepIntent {
            ordinal,
            stage,
            local_oid: local,
            incoming_oid: incoming,
            baseline_tree_oid: local_commit.tree_id(),
            baseline_index_digest: index_digest(local_commit.tree_id()),
        };
        drop(local_commit);
        // Persist the exact ordered parents before even preparing a candidate;
        // a restart must never synthesize another candidate for this stage.
        service.prepare_synchronization_integration_in_window(
            root,
            owner,
            window.number,
            &intent,
        )?;
        let prepared = if classification == merge::IntegrationDisposition::MergeRequired {
            Some(prepare_clean_merge(&path, local, incoming)?)
        } else {
            None
        };
        match classification {
            merge::IntegrationDisposition::Equal
            | merge::IntegrationDisposition::IncomingAlreadyIntegrated => {
                service.begin_synchronization_integration_effect_in_window(
                    root,
                    owner,
                    window.number,
                    ordinal,
                    None,
                )?;
                let tree = target_repository
                    .find_commit(local)
                    .map_err(|_| SynchronizationError::RecoveryRequired)?
                    .tree_id();
                service.observe_synchronization_integration_effect_in_window(
                    root,
                    owner,
                    window.number,
                    ordinal,
                    local,
                    tree,
                )?;
            }
            merge::IntegrationDisposition::FastForward => {
                let _lease = repository_lease(
                    &target_repository,
                    root,
                    RepositoryOperation::RepositorySnapshot,
                )?;
                #[cfg(test)]
                super::observation_tests::checkpoint(RemoteOperationSafePoint::BeforeLocalMutation);
                service.synchronization_boundary(root, owner)?;
                recheck_divergence_inputs(service, &input, local)?;
                if target_repository.find_commit(incoming).is_err() {
                    return Err(SynchronizationError::ExternalChange);
                }
                service.begin_synchronization_integration_effect_in_window(
                    root,
                    owner,
                    window.number,
                    ordinal,
                    None,
                )?;
                let branch = target_repository
                    .find_reference("HEAD")
                    .ok()
                    .and_then(|head| head.symbolic_target().map(str::to_owned))
                    .ok_or(SynchronizationError::ExternalChange)?;
                fast_forward(&target_repository, &branch, local, incoming)?;
                let tree = target_repository
                    .find_commit(incoming)
                    .map_err(|_| SynchronizationError::RecoveryRequired)?
                    .tree_id();
                service.observe_synchronization_integration_effect_in_window(
                    root,
                    owner,
                    window.number,
                    ordinal,
                    incoming,
                    tree,
                )?;
                local = incoming;
            }
            merge::IntegrationDisposition::MergeRequired => {
                let prepared = match prepared.ok_or(SynchronizationError::RecoveryRequired)? {
                    MergePreparation::Clean(prepared) => Some(prepared),
                    MergePreparation::Conflict => None,
                };
                let mut repository = git2::Repository::open(&path)
                    .map_err(|_| SynchronizationError::RecoveryRequired)?;
                let _lease =
                    repository_lease(&repository, root, RepositoryOperation::RepositorySnapshot)?;
                #[cfg(test)]
                super::observation_tests::checkpoint(RemoteOperationSafePoint::BeforeLocalMutation);
                service.synchronization_boundary(root, owner)?;
                recheck_divergence_inputs(service, &input, local)?;
                if repository.find_commit(incoming).is_err() {
                    return Err(SynchronizationError::ExternalChange);
                }
                if let Some(prepared) = prepared {
                    let identity = committing_identity(service, root, owner, &repository, request)?;
                    let tree = import_prepared_tree(&repository, &path, &prepared)?;
                    let local_commit = repository
                        .find_commit(local)
                        .map_err(|_| SynchronizationError::RecoveryRequired)?;
                    let incoming_commit = repository
                        .find_commit(incoming)
                        .map_err(|_| SynchronizationError::RecoveryRequired)?;
                    let tree = repository
                        .find_tree(tree)
                        .map_err(|_| SynchronizationError::RecoveryRequired)?;
                    let signature = git2::Signature::now(&identity.name, &identity.email)
                        .map_err(|_| SynchronizationError::RecoveryRequired)?;
                    let subject = match stage {
                        merge::IntegrationStage::Context => {
                            format!("Merge remote context {}", request.operation_id)
                        }
                        merge::IntegrationStage::Primary
                            if matches!(target, SynchronizationTarget::Primary) =>
                        {
                            "Merge remote primary".into()
                        }
                        merge::IntegrationStage::Primary => match target {
                            SynchronizationTarget::Context { kind, item_id } => format!(
                                "Merge primary into {} {item_id}",
                                authoring_kind_segment(kind)
                            ),
                            SynchronizationTarget::Primary => unreachable!(),
                        },
                    };
                    let candidate = repository
                        .commit(
                            None,
                            &signature,
                            &signature,
                            &subject,
                            &tree,
                            &[&local_commit, &incoming_commit],
                        )
                        .map_err(|_| SynchronizationError::RecoveryRequired)?;
                    service.begin_synchronization_integration_effect_in_window(
                        root,
                        owner,
                        window.number,
                        ordinal,
                        Some(candidate),
                    )?;
                    let branch = repository
                        .find_reference("HEAD")
                        .ok()
                        .and_then(|head| head.symbolic_target().map(str::to_owned))
                        .ok_or(SynchronizationError::ExternalChange)?;
                    fast_forward(&repository, &branch, local, candidate)?;
                    service.observe_synchronization_integration_effect_in_window(
                        root,
                        owner,
                        window.number,
                        ordinal,
                        candidate,
                        tree.id(),
                    )?;
                    local = candidate;
                } else {
                    service.begin_synchronization_integration_effect_in_window(
                        root,
                        owner,
                        window.number,
                        ordinal,
                        None,
                    )?;
                    let annotated = repository
                        .find_annotated_commit(incoming)
                        .map_err(|_| SynchronizationError::RecoveryRequired)?;
                    let mut checkout = git2::build::CheckoutBuilder::new();
                    checkout.safe().overwrite_ignored(false);
                    repository
                        .merge(&[&annotated], None, Some(&mut checkout))
                        .map_err(|_| SynchronizationError::RecoveryRequired)?;
                    drop(annotated);
                    let fingerprint =
                        integration_conflict_digest(&mut repository, window.number, ordinal)?;
                    service.release_synchronization_conflict_in_window(
                        root,
                        owner,
                        window.number,
                        ordinal,
                        fingerprint,
                    )?;
                    return Err(SynchronizationError::ConflictPending {
                        target: target.clone(),
                        operation_id: request.operation_id,
                        stage: stage_name(stage),
                    });
                }
            }
        }
    }
    Ok(local)
}

fn graph_plan(
    repository: &git2::Repository,
    target: &SynchronizationTarget,
    local: git2::Oid,
    primary: Option<git2::Oid>,
    context: Option<git2::Oid>,
    publication: RemotePublicationEvidence,
) -> Result<refs::CleanIntegrationPlan, SynchronizationError> {
    refs::plan_clean_integration(target, local, primary, context, publication, |a, b| {
        repository.graph_descendant_of(b, a)
    })
    .map_err(|error| match error {
        refs::CleanIntegrationError::PrimaryMissing => SynchronizationError::PrimaryMissing,
        refs::CleanIntegrationError::RemoteContextDeleted => {
            SynchronizationError::RemoteContextDeleted
        }
        refs::CleanIntegrationError::HistoryUnknown => SynchronizationError::HistoryUnknown,
        refs::CleanIntegrationError::MergeRequired => SynchronizationError::MergeRequired {
            target: target.clone(),
        },
        refs::CleanIntegrationError::Ancestry(_) => SynchronizationError::RecoveryRequired,
    })
}
impl RepositoryService {
    fn synchronization_boundary(
        &self,
        root: &Path,
        owner: &RemoteReservation,
    ) -> Result<(), SynchronizationError> {
        decision(self.check_synchronization_requests(root, owner)?)
    }
    fn synchronization_point(
        &self,
        root: &Path,
        owner: &RemoteReservation,
        point: RemoteOperationSafePoint,
    ) -> Result<(), SynchronizationError> {
        #[cfg(test)]
        super::observation_tests::checkpoint(point);
        decision(self.remote_safe_point(root, owner, point)?)
    }
    fn inspect_synchronization_local(
        &self,
        root: &Path,
        primary: &str,
        target: &SynchronizationTarget,
    ) -> Result<git2::Oid, SynchronizationError> {
        let repository =
            git2::Repository::open(root).map_err(|_| SynchronizationError::RecoveryRequired)?;
        let _lease = repository_lease(&repository, root, RepositoryOperation::RepositorySnapshot)?;
        local_oid(&local_target(root, primary, target)?)
    }
    fn synchronization_configuration_matches(
        &self,
        root: &Path,
        plan: &RemoteRefPlan,
        expected: &super::observation::ObservationConfiguration,
    ) -> Result<bool, SynchronizationError> {
        let ConfigurationInspection::Valid(config) = read_configuration(root)? else {
            return Ok(false);
        };
        let actual = config.publication_remote.as_ref().and_then(|remote| {
            RemoteRefPlan::from_configuration(remote, &config.primary_branch).ok()
        });
        Ok(actual.as_ref() == Some(plan)
            && self.observation_configuration(root, plan)? == *expected)
    }

    /// Bind every authenticated scope to one action snapshot, including after
    /// credentials and safe-point hooks. The adapter pins its actual handle too.
    fn with_synchronization_remote<P: SessionCredentialProvider, T>(
        &self,
        request: VerifySshTransportRequest,
        session: &mut SessionCredentials<P>,
        owner: &RemoteReservation,
        plan: &RemoteRefPlan,
        expected: &super::observation::ObservationConfiguration,
        operation: impl FnOnce(
            &mut crate::repository::transport::AuthenticatedSshRemote<'_, '_>,
        ) -> Result<T, SshTransportError>,
    ) -> Result<T, SynchronizationError> {
        self.synchronization_boundary(&request.root, owner)?;
        if !self.synchronization_configuration_matches(&request.root, plan, expected)? {
            return Err(SynchronizationError::ExternalChange);
        }
        let expectation = expected
            .transport_expectation(request.direction)
            .ok_or(SynchronizationError::RecoveryRequired)?;
        let root = request.root.clone();
        #[cfg(test)]
        crate::repository::transport::operation_tests::checkpoint(
            crate::repository::transport::operation_tests::Checkpoint::ActionSnapshotChecked,
        );
        self.with_authenticated_remote_expected(request, session, &expectation, |remote| {
            if let Err(error) = self.synchronization_boundary(&root, owner) {
                return Ok(Err(error));
            }
            match self.synchronization_configuration_matches(&root, plan, expected) {
                Ok(true) => {}
                Ok(false) => return Ok(Err(SynchronizationError::ExternalChange)),
                Err(error) => return Ok(Err(error)),
            }
            remote.require_action_expectation(&expectation)?;
            let result = operation(remote)?;
            remote.require_action_expectation(&expectation)?;
            if let Err(error) = self.synchronization_boundary(&root, owner) {
                return Ok(Err(error));
            }
            match self.synchronization_configuration_matches(&root, plan, expected) {
                Ok(true) => {}
                Ok(false) => return Ok(Err(SynchronizationError::ExternalChange)),
                Err(error) => return Ok(Err(error)),
            }
            Ok(Ok(result))
        })?
    }

    fn inspect_conflict_target(
        root: &Path,
        target: &SynchronizationTarget,
    ) -> Result<git2::Repository, SynchronizationError> {
        let ConfigurationInspection::Valid(config) = read_configuration(root)? else {
            return Err(SynchronizationError::RecoveryRequired);
        };
        materialized_target(root, &config.primary_branch, target)
            .map_err(|_| SynchronizationError::ExternalChange)
    }

    fn conflict_configuration(root: &Path) -> Result<[u8; 32], SynchronizationError> {
        let bytes = std::fs::read(root.join(".manyhands/config.toml"))
            .map_err(|_| SynchronizationError::RecoveryRequired)?;
        Ok(*blake3::hash(&bytes).as_bytes())
    }

    /// Classify from immutable index entries, never marker text or a worktree
    /// path. This is intentionally conservative: any path, mode, encoding, or
    /// canonical identity ambiguity makes the whole merge external-only.
    fn classify_conflict_entry(
        repository: &git2::Repository,
        conflict: &git2::IndexConflict,
    ) -> merge::ConflictCandidate {
        let entries = [
            conflict.ancestor.as_ref(),
            conflict.our.as_ref(),
            conflict.their.as_ref(),
        ];
        let Some(path) = entries.iter().flatten().next().map(|entry| &entry.path) else {
            return merge::ConflictCandidate {
                kind: merge::ConflictEntryKind::Noncanonical,
                structure: merge::ConflictStructure::Delete,
            };
        };
        if entries.iter().any(Option::is_none) {
            return merge::ConflictCandidate {
                kind: merge::ConflictEntryKind::Noncanonical,
                structure: merge::ConflictStructure::Delete,
            };
        }
        if entries.iter().flatten().any(|entry| entry.path != *path) {
            return merge::ConflictCandidate {
                kind: merge::ConflictEntryKind::Noncanonical,
                structure: merge::ConflictStructure::Rename,
            };
        }
        if entries.iter().flatten().any(|entry| entry.mode != 0o100644) {
            let structure = if entries.iter().flatten().any(|entry| entry.mode == 0o120000) {
                merge::ConflictStructure::Symlink
            } else {
                merge::ConflictStructure::Executable
            };
            return merge::ConflictCandidate {
                kind: merge::ConflictEntryKind::Noncanonical,
                structure,
            };
        }
        let Ok(path) = std::str::from_utf8(path) else {
            return merge::ConflictCandidate {
                kind: merge::ConflictEntryKind::Noncanonical,
                structure: merge::ConflictStructure::Binary,
            };
        };
        if std::path::Path::new(path)
            .components()
            .any(|part| !matches!(part, std::path::Component::Normal(_)))
        {
            return merge::ConflictCandidate {
                kind: merge::ConflictEntryKind::Noncanonical,
                structure: merge::ConflictStructure::Rename,
            };
        }
        let parsed = entries
            .iter()
            .flatten()
            .map(|entry| {
                repository.find_blob(entry.id).ok().and_then(|blob| {
                    std::str::from_utf8(blob.content()).ok().and_then(|text| {
                        canonical::parse_item(std::path::Path::new(path), text).ok()
                    })
                })
            })
            .collect::<Option<Vec<_>>>();
        let Some(parsed) = parsed else {
            return merge::ConflictCandidate {
                kind: merge::ConflictEntryKind::Noncanonical,
                structure: merge::ConflictStructure::Binary,
            };
        };
        let Some(first) = parsed.first() else {
            return merge::ConflictCandidate {
                kind: merge::ConflictEntryKind::Noncanonical,
                structure: merge::ConflictStructure::Delete,
            };
        };
        let identity = match first {
            canonical::CanonicalItem::Document(value) => &value.id,
            canonical::CanonicalItem::Ticket(value) => &value.id,
            canonical::CanonicalItem::Comment(value) => &value.id,
        };
        if parsed.iter().any(|item| {
            let same_kind = matches!(
                (first, item),
                (
                    canonical::CanonicalItem::Document(_),
                    canonical::CanonicalItem::Document(_)
                ) | (
                    canonical::CanonicalItem::Ticket(_),
                    canonical::CanonicalItem::Ticket(_)
                ) | (
                    canonical::CanonicalItem::Comment(_),
                    canonical::CanonicalItem::Comment(_)
                )
            );
            !same_kind
                || match item {
                    canonical::CanonicalItem::Document(value) => &value.id != identity,
                    canonical::CanonicalItem::Ticket(value) => &value.id != identity,
                    canonical::CanonicalItem::Comment(value) => &value.id != identity,
                }
        }) {
            return merge::ConflictCandidate {
                kind: merge::ConflictEntryKind::Noncanonical,
                structure: merge::ConflictStructure::IdentityChanged,
            };
        }
        let kind = match first {
            canonical::CanonicalItem::Document(_) => merge::ConflictEntryKind::Document,
            canonical::CanonicalItem::Ticket(_) => merge::ConflictEntryKind::Ticket,
            canonical::CanonicalItem::Comment(_) => merge::ConflictEntryKind::Comment,
        };
        merge::ConflictCandidate {
            kind,
            structure: merge::ConflictStructure::RegularUtf8SamePath,
        }
    }

    fn canonical_item_id(item: &canonical::CanonicalItem) -> canonical::ItemId {
        match item {
            canonical::CanonicalItem::Document(value) => value.id.clone(),
            canonical::CanonicalItem::Ticket(value) => value.id.clone(),
            canonical::CanonicalItem::Comment(value) => value.id.clone(),
        }
    }

    fn preserves_item_invariants(
        expected: &canonical::CanonicalItem,
        result: &canonical::CanonicalItem,
    ) -> bool {
        match (expected, result) {
            (canonical::CanonicalItem::Document(_), canonical::CanonicalItem::Document(_)) => true,
            (canonical::CanonicalItem::Ticket(old), canonical::CanonicalItem::Ticket(new)) => {
                old.closed_at.is_none()
                    || (old.closed_at == new.closed_at && old.closed_by == new.closed_by)
            }
            (canonical::CanonicalItem::Comment(old), canonical::CanonicalItem::Comment(new)) => {
                old.item_id == new.item_id
                    && old.parent_id == new.parent_id
                    && old.created_at == new.created_at
                    // Creator and legacy author metadata are immutable comment
                    // provenance, including their absence on legacy comments.
                    && old.unknown.get("created_by") == new.unknown.get("created_by")
                    && old.unknown.get("author") == new.unknown.get("author")
            }
            _ => false,
        }
    }

    fn validates_resolution_sides(
        repository: &git2::Repository,
        token: &merge::ConflictPathToken,
        bytes: &[u8],
    ) -> Result<bool, SynchronizationError> {
        let Ok(text) = std::str::from_utf8(bytes) else {
            return Ok(false);
        };
        let path = Path::new(std::str::from_utf8(&token.path).unwrap_or_default());
        let Ok(item) = canonical::parse_item(path, text) else {
            return Ok(false);
        };
        let side_ids = [token.base, token.local, token.incoming];
        if side_ids.iter().all(Option::is_none) {
            return Ok(false);
        }
        // No side is authoritative over another: the caller result must
        // retain every recorded immutable value. Disagreement therefore
        // requires external recovery, even when the base is still open.
        for side_oid in side_ids.into_iter().flatten() {
            let blob = repository
                .find_blob(side_oid)
                .map_err(|_| SynchronizationError::RecoveryRequired)?;
            let Ok(side_text) = std::str::from_utf8(blob.content()) else {
                return Ok(false);
            };
            let Ok(expected) = canonical::parse_item(path, side_text) else {
                return Ok(false);
            };
            if Self::canonical_item_id(&item) != Self::canonical_item_id(&expected)
                || !Self::preserves_item_invariants(&expected, &item)
            {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// Validate the prospective merged canonical context without a common-Git
    /// lease. The live index supplies every Git-selected non-conflict entry;
    /// callers supply only the observed conflict replacements.
    fn validates_prospective_context(
        repository: &git2::Repository,
        replacements: &std::collections::BTreeMap<
            u32,
            (&merge::ConflictPathToken, &merge::RedactedConflictBytes),
        >,
    ) -> Result<bool, SynchronizationError> {
        let mut sources = std::collections::BTreeMap::<Vec<u8>, Vec<u8>>::new();
        let index = repository
            .index()
            .map_err(|_| SynchronizationError::RecoveryRequired)?;
        for entry in index.iter() {
            if entry.flags >> 12 & 3 != 0 {
                continue;
            }
            let Ok(path) = std::str::from_utf8(&entry.path) else {
                continue;
            };
            let Ok(blob) = repository.find_blob(entry.id) else {
                return Err(SynchronizationError::RecoveryRequired);
            };
            if let Ok(text) = std::str::from_utf8(blob.content())
                && canonical::parse_item(std::path::Path::new(path), text).is_ok()
            {
                sources.insert(entry.path.clone(), blob.content().to_vec());
            }
        }
        // The recorded Git-selected stage-zero entries are the baseline, with
        // observed local (or available immutable) conflict sides substituted.
        // Missing conflict entries must not manufacture baseline orphan errors.
        let mut baseline = sources.clone();
        for (token, _) in replacements.values() {
            if let Some(oid) = token.local.or(token.incoming).or(token.base) {
                let blob = repository
                    .find_blob(oid)
                    .map_err(|_| SynchronizationError::RecoveryRequired)?;
                baseline.insert(token.path.clone(), blob.content().to_vec());
            }
        }
        let mut affected = std::collections::BTreeSet::new();
        for (token, bytes) in replacements.values() {
            let Some(item) = std::str::from_utf8(&token.path).ok().and_then(|path| {
                std::str::from_utf8(bytes.bytes())
                    .ok()
                    .and_then(|text| canonical::parse_item(Path::new(path), text).ok())
            }) else {
                return Ok(false);
            };
            affected.insert(Self::canonical_item_id(&item));
            sources.insert(token.path.clone(), bytes.bytes().to_vec());
        }
        Ok(Self::validates_context_sources(
            &baseline, &sources, affected,
        ))
    }

    fn validates_context_sources(
        baseline: &std::collections::BTreeMap<Vec<u8>, Vec<u8>>,
        sources: &std::collections::BTreeMap<Vec<u8>, Vec<u8>>,
        mut affected: std::collections::BTreeSet<canonical::ItemId>,
    ) -> bool {
        let context_sources = |sources: &std::collections::BTreeMap<Vec<u8>, Vec<u8>>| {
            sources
                .iter()
                .filter_map(|(path, bytes)| {
                    Some((
                        PathBuf::from(std::str::from_utf8(path).ok()?),
                        std::str::from_utf8(bytes).ok()?.to_owned(),
                    ))
                })
                .collect::<Vec<_>>()
        };
        let prospective_sources = context_sources(sources);
        let items = prospective_sources
            .iter()
            .filter_map(|(path, text)| {
                canonical::parse_item(path, text)
                    .ok()
                    .map(|item| (path.clone(), item))
            })
            .collect::<Vec<_>>();
        // Close over item/thread dependencies in both directions, including
        // nonconforming candidates omitted from ValidatedContext.items. This
        // rejects inherited problems affecting touched identities or parents.
        loop {
            let before = affected.len();
            for (_, item) in &items {
                if let canonical::CanonicalItem::Comment(comment) = item
                    && (affected.contains(&comment.id)
                        || affected.contains(&comment.item_id)
                        || comment
                            .parent_id
                            .as_ref()
                            .is_some_and(|id| affected.contains(id)))
                {
                    affected.insert(comment.id.clone());
                    affected.insert(comment.item_id.clone());
                    affected.extend(comment.parent_id.iter().cloned());
                }
            }
            if before == affected.len() {
                break;
            }
        }
        let baseline_context = canonical::validate_context(context_sources(baseline));
        let context = canonical::validate_context(prospective_sources);
        context.problems.iter().all(|problem| {
            let path = preflight_path_bytes(&problem.path);
            baseline.get(&path) == sources.get(&path)
                && baseline_context.problems.contains(problem)
                && !items.iter().any(|(path, item)| {
                    path == &problem.path && affected.contains(&Self::canonical_item_id(item))
                })
        })
    }

    /// An external repair is a whole committed merge, including unsupported
    /// code conflicts. Read only its ODB tree; never stage external code or
    /// interpret a subject/authoring journal as commit authority.
    fn validates_external_integration(
        repository: &git2::Repository,
        step: &state::IntegrationStepEvidence,
        candidate: git2::Oid,
    ) -> Result<bool, SynchronizationError> {
        let local = repository
            .find_commit(step.intent.local_oid)
            .map_err(|_| SynchronizationError::RecoveryRequired)?;
        let incoming = repository
            .find_commit(step.intent.incoming_oid)
            .map_err(|_| SynchronizationError::RecoveryRequired)?;
        let base = repository
            .merge_base(local.id(), incoming.id())
            .and_then(|oid| repository.find_commit(oid))
            .map_err(|_| SynchronizationError::RecoveryRequired)?;
        let result = repository
            .find_commit(candidate)
            .map_err(|_| SynchronizationError::RecoveryRequired)?;
        let result_tree = result
            .tree()
            .map_err(|_| SynchronizationError::RecoveryRequired)?;
        let sides = [base.tree(), local.tree(), incoming.tree()]
            .into_iter()
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| SynchronizationError::RecoveryRequired)?;
        let baseline = Self::canonical_tree_sources(repository, &sides[1])?;
        let prospective = Self::canonical_tree_sources(repository, &result_tree)?;
        for (path, shape) in &prospective.entries {
            if baseline.entries.get(path) == Some(shape) {
                // Preserve genuinely unchanged unrelated baseline problems,
                // including invalid encoding and unsupported legacy shapes.
                continue;
            }
            if std::str::from_utf8(path).is_err()
                || shape.1 != 0o100644
                || shape.2 != Some(git2::ObjectType::Blob)
                || prospective
                    .sources
                    .get(path)
                    .is_none_or(|bytes| std::str::from_utf8(bytes).is_err())
            {
                return Ok(false);
            }
        }
        let mut changed = std::collections::BTreeSet::new();
        for side in &sides {
            let diff = repository
                .diff_tree_to_tree(Some(side), Some(&result_tree), None)
                .map_err(|_| SynchronizationError::RecoveryRequired)?;
            for delta in diff.deltas() {
                for file in [delta.old_file(), delta.new_file()] {
                    if let Some(path) = file.path() {
                        changed.insert(path.to_owned());
                    }
                }
            }
        }
        let parse = |tree: &git2::Tree<'_>, path: &Path| {
            tree.get_path(path).ok().and_then(|entry| {
                repository.find_blob(entry.id()).ok().and_then(|blob| {
                    std::str::from_utf8(blob.content())
                        .ok()
                        .and_then(|text| canonical::parse_item(path, text).ok())
                })
            })
        };
        let mut affected = std::collections::BTreeSet::new();
        for path in changed {
            let result_item = parse(&result_tree, &path);
            for side in &sides {
                if let Some(old) = parse(side, &path) {
                    let Some(new) = &result_item else {
                        return Ok(false);
                    };
                    if Self::canonical_item_id(&old) != Self::canonical_item_id(new)
                        || !Self::preserves_item_invariants(&old, new)
                    {
                        return Ok(false);
                    }
                    affected.insert(Self::canonical_item_id(&old));
                }
            }
            if let Some(item) = result_item {
                affected.insert(Self::canonical_item_id(&item));
            }
        }
        Ok(Self::validates_context_sources(
            &Self::canonical_context_tree_sources(&baseline),
            &Self::canonical_context_tree_sources(&prospective),
            affected,
        ))
    }

    fn canonical_tree_sources(
        repository: &git2::Repository,
        tree: &git2::Tree<'_>,
    ) -> Result<CanonicalTreeSnapshot, SynchronizationError> {
        let mut sources = std::collections::BTreeMap::new();
        let mut entries = std::collections::BTreeMap::new();
        let mut failed = false;
        tree.walk(git2::TreeWalkMode::PreOrder, |parent, entry| {
            let mut path = parent.as_bytes().to_vec();
            path.extend_from_slice(entry.name_bytes());
            let path_text = std::str::from_utf8(&path).ok();
            let canonical_path = (path.ends_with(b".md")
                && (path.starts_with(b"docs/")
                    || path.starts_with(b".manyhands/tickets/")
                    || path.starts_with(b".manyhands/comments/")))
                || path_text
                    .is_some_and(|path| Self::is_canonical_tree_source_path(Path::new(path)));
            if canonical_path {
                entries.insert(path.clone(), (entry.id(), entry.filemode(), entry.kind()));
            }
            if entry.kind() != Some(git2::ObjectType::Blob) {
                return git2::TreeWalkResult::Ok;
            }
            match repository.find_blob(entry.id()) {
                Ok(blob) => {
                    if canonical_path
                        || path_text.is_some_and(|path| {
                            std::str::from_utf8(blob.content())
                                .ok()
                                .is_some_and(|text| {
                                    Self::is_canonical_tree_source(Path::new(path), text)
                                })
                        })
                    {
                        entries.insert(path.clone(), (entry.id(), entry.filemode(), entry.kind()));
                        sources.insert(path, blob.content().to_vec());
                    }
                }
                Err(_) => failed = true,
            }
            git2::TreeWalkResult::Ok
        })
        .map_err(|_| SynchronizationError::RecoveryRequired)?;
        if failed {
            return Err(SynchronizationError::RecoveryRequired);
        }
        Ok(CanonicalTreeSnapshot { sources, entries })
    }

    /// Called after shape/encoding comparison: any undecodable source omitted
    /// here has already been proved byte-and-shape-identical to the baseline.
    fn canonical_context_tree_sources(
        snapshot: &CanonicalTreeSnapshot,
    ) -> std::collections::BTreeMap<Vec<u8>, Vec<u8>> {
        snapshot
            .sources
            .iter()
            .filter_map(|(path, bytes)| {
                let path_text = std::str::from_utf8(path).ok()?;
                let text = std::str::from_utf8(bytes).ok()?;
                Self::is_canonical_tree_source(Path::new(path_text), text)
                    .then(|| (path.clone(), bytes.clone()))
            })
            .collect()
    }

    fn is_canonical_tree_source_path(path: &Path) -> bool {
        canonical::item_path_kind(path).is_ok()
            || ((path.starts_with(".manyhands/tickets") || path.starts_with(".manyhands/comments"))
                && path.extension().is_some_and(|extension| extension == "md"))
    }

    fn is_canonical_tree_source(path: &Path, text: &str) -> bool {
        if path.starts_with(".manyhands/tickets") || path.starts_with(".manyhands/comments") {
            return true;
        }
        // Match discovery's declared-managed document boundary without reading
        // any live or caller-supplied path. Include malformed declared items so
        // the affected-context gate can reject new diagnostics.
        let normalized = text.replace("\r\n", "\n");
        let Some(front_matter) = normalized
            .strip_prefix("---\n")
            .and_then(|text| text.split_once("\n---").map(|(header, _)| header))
        else {
            return false;
        };
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

    /// Inspect only the exact conflict retained by this synchronization. This
    /// reads actual Git state and never treats a foreign merge as owned.
    pub fn inspect_synchronization_recovery(
        &self,
        root: &Path,
        operation_id: OperationId,
    ) -> Result<SynchronizationConflictInspection, SynchronizationError> {
        let (_, root) = canonical_repository_root(root, RepositoryOperation::RepositorySnapshot)?;
        let record = state::with_transaction(self, &root, |tx, id| {
            state::read_operation(tx, id, operation_id)
        })?
        .ok_or(SynchronizationError::RecoveryRequired)?;
        let target = match record.target.action() {
            RemoteOperationAction::SynchronizePrimary => SynchronizationTarget::Primary,
            RemoteOperationAction::SynchronizeContext => {
                let (kind, item_id) = record.target.item().ok_or_else(state::recovery_required)?;
                SynchronizationTarget::Context {
                    kind,
                    item_id: item_id.clone(),
                }
            }
            _ => return Err(SynchronizationError::RecoveryRequired),
        };
        // State reads are bounded to this operation; exactly one retained
        // conflict-pending child is required for an inspectable conflict.
        let step = state::with_transaction(self, &root, |tx, _| {
            let mut pending = None;
            let window = state::latest_integration_window(tx, &record)?;
            for ordinal in 0..=1 {
                if let Some(step) =
                    state::integration_step_in_window(tx, record.id, window.number, ordinal)?
                    && matches!(
                        step.phase,
                        state::IntegrationStepPhase::ConflictPending
                            | state::IntegrationStepPhase::ResolutionPrepared
                            | state::IntegrationStepPhase::CommitPrepared
                    )
                {
                    if pending.is_some() {
                        return Err(state::recovery_required());
                    }
                    pending = Some((ordinal, step));
                }
            }
            pending.ok_or_else(state::recovery_required)
        })?;
        let (ordinal, step) = step;
        let mut repository = Self::inspect_conflict_target(&root, &target)?;
        let _lease = repository_lease(&repository, &root, RepositoryOperation::RepositorySnapshot)?;
        let head = local_oid(&repository)?;
        let fingerprint =
            integration_conflict_digest(&mut repository, step.window_number, ordinal)?;
        if head != step.intent.local_oid
            || fingerprint
                != step
                    .conflict_digest
                    .ok_or(SynchronizationError::RecoveryRequired)?
        {
            return Err(SynchronizationError::ExternalChange);
        }
        let configuration = Self::conflict_configuration(&root)?;
        let observation = merge::ConflictObservation {
            operation_id,
            window_number: step.window_number,
            ordinal,
            fingerprint,
            head,
            configuration,
            root: root.clone(),
        };
        let index = repository
            .index()
            .map_err(|_| SynchronizationError::RecoveryRequired)?;
        let mut tokens = Vec::new();
        let mut candidates = Vec::new();
        for (entry_ordinal, conflict) in index
            .conflicts()
            .map_err(|_| SynchronizationError::RecoveryRequired)?
            .enumerate()
        {
            let conflict = conflict.map_err(|_| SynchronizationError::RecoveryRequired)?;
            let entry = conflict
                .our
                .as_ref()
                .or(conflict.their.as_ref())
                .or(conflict.ancestor.as_ref())
                .ok_or(SynchronizationError::RecoveryRequired)?;
            candidates.push(Self::classify_conflict_entry(&repository, &conflict));
            tokens.push(merge::ConflictPathToken {
                observation: observation.clone(),
                ordinal: entry_ordinal as u32,
                path: entry.path.clone(),
                base: conflict.ancestor.as_ref().map(|entry| entry.id),
                base_mode: conflict.ancestor.as_ref().map(|entry| entry.mode),
                local: conflict.our.as_ref().map(|entry| entry.id),
                local_mode: conflict.our.as_ref().map(|entry| entry.mode),
                incoming: conflict.their.as_ref().map(|entry| entry.id),
                incoming_mode: conflict.their.as_ref().map(|entry| entry.mode),
            });
        }
        let eligibility = merge::conflict_eligibility(&candidates);
        let paths = tokens
            .into_iter()
            .map(|token| SynchronizationConflictPath { token, eligibility })
            .collect();
        Ok(SynchronizationConflictInspection {
            operation_id,
            target,
            stage: stage_name(step.intent.stage),
            local_parent: step.intent.local_oid,
            incoming_parent: step.intent.incoming_oid,
            observation,
            paths,
        })
    }

    /// Explicitly read the three Git index sides for an inspection-issued token.
    /// Every optimistic precondition is re-observed first; no content is saved.
    pub fn read_synchronization_conflict(
        &self,
        token: &merge::ConflictPathToken,
    ) -> Result<EphemeralSynchronizationConflictSides, SynchronizationError> {
        let inspection = self.inspect_synchronization_recovery(
            &token.observation.root,
            token.observation.operation_id,
        )?;
        if inspection.observation != token.observation
            || !inspection.paths.iter().any(|path| path.token == *token)
        {
            return Err(SynchronizationError::ExternalChange);
        }
        let safe_path = std::str::from_utf8(&token.path).ok().filter(|path| {
            !path.is_empty()
                && std::path::Path::new(path)
                    .components()
                    .all(|component| matches!(component, std::path::Component::Normal(_)))
        });
        if safe_path.is_none()
            || [token.base_mode, token.local_mode, token.incoming_mode]
                .into_iter()
                .flatten()
                .any(|mode| mode != 0o100644)
        {
            return Err(SynchronizationError::ExternalResolutionRequired {
                target: inspection.target,
                operation_id: token.observation.operation_id,
            });
        }
        let mut repository =
            Self::inspect_conflict_target(&token.observation.root, &inspection.target)?;
        let _lease = repository_lease(
            &repository,
            &token.observation.root,
            RepositoryOperation::RepositorySnapshot,
        )?;
        if local_oid(&repository)? != token.observation.head
            || Self::conflict_configuration(&token.observation.root)?
                != token.observation.configuration
            || integration_conflict_digest(
                &mut repository,
                token.observation.window_number,
                token.observation.ordinal,
            )? != token.observation.fingerprint
        {
            return Err(SynchronizationError::ExternalChange);
        }
        let read = |oid: Option<git2::Oid>| -> Result<Option<merge::RedactedConflictBytes>, SynchronizationError> {
            oid.map(|oid| {
                let blob = repository.find_blob(oid).map_err(|_| SynchronizationError::RecoveryRequired)?;
                if blob.is_binary() {
                    return Err(SynchronizationError::ExternalResolutionRequired {
                        target: inspection.target.clone(),
                        operation_id: token.observation.operation_id,
                    });
                }
                Ok(merge::RedactedConflictBytes::from_bytes(blob.content().to_vec()))
            }).transpose()
        };
        // Current worktree bytes are deliberately unavailable: a path-based
        // read could follow a post-inspection symlink. The three immutable index
        // stages above are the complete safe inspection surface.
        let sides = EphemeralSynchronizationConflictSides {
            base: read(token.base)?,
            local: read(token.local)?,
            incoming: read(token.incoming)?,
            current: None,
        };
        if [&sides.base, &sides.local, &sides.incoming]
            .into_iter()
            .flatten()
            .any(|bytes| std::str::from_utf8(bytes.bytes()).is_err())
        {
            return Err(SynchronizationError::ExternalResolutionRequired {
                target: inspection.target,
                operation_id: token.observation.operation_id,
            });
        }
        Ok(sides)
    }

    fn reconcile_resolution_candidate(
        &self,
        root: &Path,
        request: &merge::ResolveSynchronizationRequest,
    ) -> Result<Option<merge::ResolveSynchronizationOutcome>, SynchronizationError> {
        let record = state::with_transaction(self, root, |tx, id| {
            state::read_operation(tx, id, request.synchronization_id)
        })?
        .ok_or(SynchronizationError::RecoveryRequired)?;
        let Some((
            step,
            candidate,
            phase,
            expected_input_digest,
            requires_confirmation,
            expected_paths,
        )) = state::with_transaction(self, root, |tx, _| {
            state::resolution_candidate_for_attempt(tx, &record, request.attempt_id)
        })?
        else {
            return Ok(None);
        };
        if request.observation.window_number != step.window_number
            || request.observation.ordinal != step.intent.ordinal
            || request.observation.operation_id != request.synchronization_id
        {
            return Err(SynchronizationError::RecoveryRequired);
        }
        // Candidate recovery accepts only the immutable, complete request that
        // created this attempt, including its identity confirmation.
        let mut input = blake3::Hasher::new();
        input.update(b"manyhands-resolution-v1\0");
        if requires_confirmation {
            let Some(identity) = &request.identity else {
                return Err(SynchronizationError::RecoveryRequired);
            };
            input.update(identity.confirmation_id.to_string().as_bytes());
            input.update(identity.identity.name.as_bytes());
            input.update(identity.identity.email.as_bytes());
        }
        let mut resolutions = request.resolutions.iter().collect::<Vec<_>>();
        resolutions.sort_by_key(|(token, _)| token.ordinal);
        if resolutions.is_empty()
            || resolutions
                .windows(2)
                .any(|pair| pair[0].0.ordinal == pair[1].0.ordinal)
            || resolutions
                .iter()
                .any(|(token, _)| token.observation != request.observation)
        {
            return Err(SynchronizationError::RecoveryRequired);
        }
        if resolutions.len() != expected_paths.len()
            || resolutions.iter().zip(&expected_paths).any(
                |((token, bytes), (ordinal, path_digest, expected_digest, result_digest))| {
                    token.ordinal != *ordinal
                        || blake3::hash(&token.path).as_bytes() != path_digest
                        || conflict_token_digest(token) != *expected_digest
                        || blake3::hash(bytes.bytes()).as_bytes() != result_digest
                },
            )
        {
            return Err(SynchronizationError::RecoveryRequired);
        }
        for (token, bytes) in resolutions {
            input.update(&token.ordinal.to_be_bytes());
            input.update(&token.path);
            input.update(&conflict_token_digest(token));
            input.update(bytes.bytes());
        }
        if *input.finalize().as_bytes() != expected_input_digest {
            return Err(SynchronizationError::RecoveryRequired);
        }
        let target = match record.target.action() {
            RemoteOperationAction::SynchronizePrimary => SynchronizationTarget::Primary,
            RemoteOperationAction::SynchronizeContext => {
                let (kind, item_id) = record
                    .target
                    .item()
                    .ok_or(SynchronizationError::RecoveryRequired)?;
                SynchronizationTarget::Context {
                    kind,
                    item_id: item_id.clone(),
                }
            }
            _ => return Err(SynchronizationError::RecoveryRequired),
        };
        let ConfigurationInspection::Valid(config) = read_configuration(root)? else {
            return Err(SynchronizationError::RecoveryRequired);
        };
        let Some(remote) = config.publication_remote.as_deref() else {
            return Err(SynchronizationError::RecoveryRequired);
        };
        let plan = RemoteRefPlan::from_configuration(remote, &config.primary_branch)
            .map_err(|_| SynchronizationError::RecoveryRequired)?;
        let operation_target = target.operation_target(&plan);
        let repository = Self::inspect_conflict_target(root, &target)?;
        // Retained candidates from an earlier process/version receive the same
        // all-side invariant gate before index/ref/retirement effects.
        for (token, bytes) in &request.resolutions {
            if !Self::validates_resolution_sides(&repository, token, bytes.bytes())? {
                return Ok(Some(merge::ResolveSynchronizationOutcome::ValidationFailed));
            }
        }
        #[cfg(any(unix, windows))]
        let mut ref_snapshot = RefLogSnapshot::read(&repository)?;
        let mut lease =
            repository_lease(&repository, root, RepositoryOperation::RepositorySnapshot)?;
        if foreign_resolution_metadata_present(&repository) {
            return Err(SynchronizationError::ExternalChange);
        }
        let owner = match self.reacquire_synchronization_conflict_in_window(
            root,
            request.synchronization_id,
            &operation_target,
            request.observation.window_number,
            request.observation.ordinal,
            request.observation.fingerprint,
        )? {
            RemoteReservationOutcome::Reserved(owner) => owner,
            RemoteReservationOutcome::Busy => return Err(SynchronizationError::Busy),
            _ => return Err(SynchronizationError::RecoveryRequired),
        };
        if Self::conflict_configuration(root)? != request.observation.configuration {
            return Err(SynchronizationError::ExternalChange);
        }
        #[cfg(any(unix, windows))]
        refuse_ambiguous_resolution_backend_locks(&repository)?;
        let mut index_lock =
            ResolutionIndexLock::acquire(&repository, self, root, &owner, request.attempt_id)?;
        #[cfg(any(unix, windows))]
        index_lock.metadata_matches(phase == "applied")?;
        let current_head = local_oid(&repository)?;
        #[cfg(any(unix, windows))]
        let ref_proof = {
            ref_snapshot.revalidate(&repository)?;
            let commit = repository
                .find_commit(candidate)
                .map_err(|_| SynchronizationError::RecoveryRequired)?;
            index_lock.bind_ref_transition(
                RefLogContext {
                    service: self,
                    root,
                    owner: &owner,
                    request,
                    input: expected_input_digest,
                },
                candidate,
                &ref_snapshot,
                &commit.committer(),
            )?
        };
        #[cfg(any(unix, windows))]
        if (current_head == candidate
            && !ref_snapshot.matches(
                ref_proof
                    .intended
                    .as_ref()
                    .ok_or(SynchronizationError::RecoveryRequired)?,
            ))
            || (current_head == step.intent.local_oid && !ref_snapshot.matches(&ref_proof.baseline))
        {
            return Err(SynchronizationError::RecoveryRequired);
        }
        if current_head != candidate {
            if phase != "candidate_prepared" || current_head != step.intent.local_oid {
                return Err(SynchronizationError::ExternalChange);
            }
            #[cfg(any(unix, windows))]
            if index_lock.artifact.output.is_none() {
                // A recorded candidate never re-enters candidate creation or
                // identity preflight. Rebuild only its private index preparation.
                let commit = repository
                    .find_commit(candidate)
                    .map_err(|_| SynchronizationError::RecoveryRequired)?;
                let tree = commit
                    .tree()
                    .map_err(|_| SynchronizationError::RecoveryRequired)?;
                let mut index = index_lock.authoritative_index()?;
                if !only_resolution_paths_are_dirty(&repository, &request.resolutions)? {
                    return Err(SynchronizationError::ExternalChange);
                }
                for (token, bytes) in &request.resolutions {
                    let relative = Path::new(
                        std::str::from_utf8(&token.path)
                            .map_err(|_| SynchronizationError::RecoveryRequired)?,
                    );
                    let entry = tree
                        .get_path(relative)
                        .map_err(|_| SynchronizationError::RecoveryRequired)?;
                    if entry.filemode() != 0o100644
                        || repository
                            .find_blob(entry.id())
                            .map_err(|_| SynchronizationError::RecoveryRequired)?
                            .content()
                            != bytes.bytes()
                        || owned_result_digest(
                            repository
                                .workdir()
                                .ok_or(SynchronizationError::RecoveryRequired)?,
                            relative,
                        )? != *blake3::hash(bytes.bytes()).as_bytes()
                    {
                        return Err(SynchronizationError::ExternalChange);
                    }
                    for stage in 1..=3 {
                        index
                            .remove(relative, stage)
                            .map_err(|_| SynchronizationError::RecoveryRequired)?;
                    }
                    index
                        .add(&git2::IndexEntry {
                            ctime: git2::IndexTime::new(0, 0),
                            mtime: git2::IndexTime::new(0, 0),
                            dev: 0,
                            ino: 0,
                            mode: 0o100644,
                            uid: 0,
                            gid: 0,
                            file_size: bytes.bytes().len() as u32,
                            id: entry.id(),
                            flags: 0,
                            flags_extended: 0,
                            path: token.path.clone(),
                        })
                        .map_err(|_| SynchronizationError::RecoveryRequired)?;
                }
                if index.has_conflicts()
                    || index
                        .write_tree_to(&repository)
                        .map_err(|_| SynchronizationError::RecoveryRequired)?
                        != tree.id()
                    || commit.parent_count() != 2
                    || commit.parent_id(0).ok() != Some(step.intent.local_oid)
                    || commit.parent_id(1).ok() != Some(step.intent.incoming_oid)
                {
                    return Err(SynchronizationError::ExternalChange);
                }
                index_lock.persist(
                    &mut index,
                    &repository,
                    self,
                    root,
                    &owner,
                    request.attempt_id,
                )?;
            }
            #[cfg(any(unix, windows))]
            index_lock.install_prepared()?;
            let commit = repository
                .find_commit(candidate)
                .map_err(|_| SynchronizationError::RecoveryRequired)?;
            let mut index = repository
                .index()
                .map_err(|_| SynchronizationError::RecoveryRequired)?;
            index
                .read(true)
                .map_err(|_| SynchronizationError::RecoveryRequired)?;
            if index.has_conflicts()
                || index
                    .write_tree_to(&repository)
                    .map_err(|_| SynchronizationError::RecoveryRequired)?
                    != commit.tree_id()
                || commit.parent_count() != 2
                || commit.parent_id(0).ok() != Some(step.intent.local_oid)
                || commit.parent_id(1).ok() != Some(step.intent.incoming_oid)
                || Self::conflict_configuration(root)? != request.observation.configuration
            {
                return Err(SynchronizationError::ExternalChange);
            }
            for (token, bytes) in &request.resolutions {
                let relative = Path::new(
                    std::str::from_utf8(&token.path)
                        .map_err(|_| SynchronizationError::RecoveryRequired)?,
                );
                if owned_result_digest(
                    repository
                        .workdir()
                        .ok_or(SynchronizationError::RecoveryRequired)?,
                    relative,
                )? != *blake3::hash(bytes.bytes()).as_bytes()
                {
                    return Err(SynchronizationError::ExternalChange);
                }
            }
            let mut options = resolution_status_options();
            if repository
                .statuses(Some(&mut options))
                .map_err(|_| SynchronizationError::RecoveryRequired)?
                .iter()
                .any(|entry| {
                    entry.status().intersects(
                        git2::Status::WT_NEW
                            | git2::Status::WT_MODIFIED
                            | git2::Status::WT_DELETED
                            | git2::Status::WT_TYPECHANGE
                            | git2::Status::WT_RENAMED
                            | git2::Status::CONFLICTED,
                    )
                })
            {
                return Err(SynchronizationError::ExternalChange);
            }
            index_lock.persisted_images_match()?;
            let name = repository
                .head()
                .map_err(|_| SynchronizationError::RecoveryRequired)?
                .name()
                .ok_or(SynchronizationError::RecoveryRequired)?
                .to_owned();
            #[cfg(any(unix, windows))]
            {
                if name != ref_snapshot.branch {
                    return Err(SynchronizationError::RecoveryRequired);
                }
                apply_resolution_ref(
                    &repository,
                    RefLogContext {
                        service: self,
                        root,
                        owner: &owner,
                        request,
                        input: expected_input_digest,
                    },
                    candidate,
                    &ref_snapshot,
                    &ref_proof,
                )?;
                // Full image hashing stays outside the short common-Git lease;
                // the reservation and stable sentinel continue fencing writers.
                drop(lease);
                #[cfg(test)]
                run_resolution_index_hook(&RESOLUTION_REF_REFRESH_HOOK, root);
                ref_snapshot = RefLogSnapshot::read(&repository)?;
                lease =
                    repository_lease(&repository, root, RepositoryOperation::RepositorySnapshot)?;
            }
            #[cfg(not(any(unix, windows)))]
            {
                let _ = name;
                return Err(SynchronizationError::RecoveryRequired);
            }
        }
        #[cfg(any(unix, windows))]
        {
            ref_snapshot.validate_final_proof(&repository, &ref_proof, candidate)?;
            index_lock.installed_image_matches()?;
            index_lock.metadata_matches(phase == "applied")?;
        }
        if foreign_resolution_metadata_present(&repository) {
            return Err(SynchronizationError::ExternalChange);
        }
        let _held_lease = lease;
        let commit = repository
            .find_commit(candidate)
            .map_err(|_| SynchronizationError::RecoveryRequired)?;
        let mut candidate_status_options = resolution_status_options();
        let candidate_dirty = repository
            .statuses(Some(&mut candidate_status_options))
            .map_err(|_| SynchronizationError::RecoveryRequired)?
            .iter()
            .any(|entry| !entry.status().is_ignored());
        if commit.parent_count() != 2
            || commit
                .parent_id(0)
                .map_err(|_| SynchronizationError::RecoveryRequired)?
                != step.intent.local_oid
            || commit
                .parent_id(1)
                .map_err(|_| SynchronizationError::RecoveryRequired)?
                != step.intent.incoming_oid
            || repository
                .index()
                .map_err(|_| SynchronizationError::RecoveryRequired)?
                .has_conflicts()
            || candidate_dirty
        {
            return Err(SynchronizationError::ExternalChange);
        }
        let tree = commit
            .tree()
            .map_err(|_| SynchronizationError::RecoveryRequired)?;
        if request.resolutions.is_empty()
            || request.resolutions.iter().any(|(token, bytes)| {
                token.observation != request.observation
                    || tree
                        .get_path(Path::new(
                            std::str::from_utf8(&token.path).unwrap_or_default(),
                        ))
                        .ok()
                        .and_then(|entry| repository.find_blob(entry.id()).ok())
                        .is_none_or(|blob| blob.content() != bytes.bytes())
            })
        {
            return Err(SynchronizationError::RecoveryRequired);
        }
        let tree_id = tree.id();
        drop(tree);
        drop(commit);
        #[cfg(any(unix, windows))]
        refuse_ambiguous_resolution_backend_locks(&repository)?;
        #[cfg(any(unix, windows))]
        {
            sync_resolution_candidate(&repository, candidate, &request.resolutions)?;
            ref_snapshot.storage_barrier(&repository)?;
            ref_snapshot.validate_final_proof(&repository, &ref_proof, candidate)?;
        }
        self.advance_synchronization_resolution_ref_effect(
            root,
            &owner,
            request.attempt_id,
            "observed",
        )?;
        if phase == "candidate_prepared" {
            self.observe_synchronization_resolution_checkpoint(
                root,
                &owner,
                request.attempt_id,
                candidate,
                tree_id,
            )?;
            self.check_failure(
                FailurePoint::ResolutionAfterCheckpointObservation,
                RepositoryOperation::RepositorySnapshot,
                root,
            )?;
        }
        self.check_failure(
            FailurePoint::ResolutionBeforeMetadataRetirement,
            RepositoryOperation::RepositorySnapshot,
            root,
        )?;
        #[cfg(any(unix, windows))]
        if index_lock.artifact.phase == "published" {
            index_lock.retire_metadata()?;
        } else {
            // Release intent is reachable only after every metadata member is absent.
            if RESOLUTION_MERGE_MEMBERS
                .iter()
                .any(|name| repository.path().join(name).exists())
            {
                return Err(SynchronizationError::ExternalChange);
            }
        }
        index_lock.retire(self, root, &owner, request.attempt_id)?;
        self.check_failure(
            FailurePoint::ResolutionAfterIndexLockRetirement,
            RepositoryOperation::RepositorySnapshot,
            root,
        )?;
        self.finalize_synchronization_resolution(root, &owner, request.attempt_id, candidate)?;
        Ok(Some(
            merge::ResolveSynchronizationOutcome::LocalCheckpointComplete {
                commit_oid: candidate,
            },
        ))
    }

    /// Resolve an observed, all-canonical conflict without transport or
    /// publication. The caller must explicitly restart synchronization later.
    pub fn resolve_synchronization(
        &self,
        request: merge::ResolveSynchronizationRequest,
    ) -> Result<merge::ResolveSynchronizationOutcome, SynchronizationError> {
        let (_, root) =
            canonical_repository_root(&request.root, RepositoryOperation::RepositorySnapshot)?;
        if root != request.observation.root
            || request.synchronization_id != request.observation.operation_id
        {
            return Ok(merge::ResolveSynchronizationOutcome::StaleObservation);
        }
        if let Some(outcome) = self.reconcile_resolution_candidate(&root, &request)? {
            return Ok(outcome);
        }
        let inspection =
            self.inspect_synchronization_recovery(&root, request.synchronization_id)?;
        if inspection.observation != request.observation
            || inspection
                .paths
                .iter()
                .any(|path| path.eligibility != merge::ConflictEligibility::EligibleCanonical)
            || request.resolutions.len() != inspection.paths.len()
        {
            return Ok(merge::ResolveSynchronizationOutcome::StaleObservation);
        }
        let preflight_repository = Self::inspect_conflict_target(&root, &inspection.target)?;
        let mut supplied = std::collections::BTreeMap::new();
        for (token, bytes) in &request.resolutions {
            if token.observation != request.observation
                || inspection.paths.iter().all(|path| path.token != *token)
                || supplied.insert(token.ordinal, (token, bytes)).is_some()
            {
                return Ok(merge::ResolveSynchronizationOutcome::ValidationFailed);
            }
            if !Self::validates_resolution_sides(&preflight_repository, token, bytes.bytes())? {
                return Ok(merge::ResolveSynchronizationOutcome::ValidationFailed);
            }
        }
        if supplied.len() != inspection.paths.len()
            || !Self::validates_prospective_context(&preflight_repository, &supplied)?
        {
            return Ok(merge::ResolveSynchronizationOutcome::ValidationFailed);
        }
        // This is deliberately taken before acquiring the durable attempt: a
        // failed local preflight must leave no attempt metadata to resume.
        #[cfg(any(unix, windows))]
        let mut ref_snapshot = RefLogSnapshot::read(&preflight_repository)?;
        let preflight_digest = resolution_preflight(&preflight_repository, &request.resolutions)?;

        let ConfigurationInspection::Valid(config) = read_configuration(&root)? else {
            return Ok(merge::ResolveSynchronizationOutcome::RecoveryRequired);
        };
        let Some(remote) = config.publication_remote.as_deref() else {
            return Ok(merge::ResolveSynchronizationOutcome::RecoveryRequired);
        };
        let plan = RemoteRefPlan::from_configuration(remote, &config.primary_branch)
            .map_err(|_| SynchronizationError::RecoveryRequired)?;
        let target = inspection.target.operation_target(&plan);
        let owner = match self.reacquire_synchronization_conflict_in_window(
            &root,
            request.synchronization_id,
            &target,
            request.observation.window_number,
            request.observation.ordinal,
            request.observation.fingerprint,
        )? {
            RemoteReservationOutcome::Reserved(owner) => owner,
            RemoteReservationOutcome::Busy => return Err(SynchronizationError::Busy),
            _ => return Ok(merge::ResolveSynchronizationOutcome::RecoveryRequired),
        };
        let mut repository = Self::inspect_conflict_target(&root, &inspection.target)?;
        let lease = repository_lease(&repository, &root, RepositoryOperation::RepositorySnapshot)?;
        #[cfg(any(unix, windows))]
        ref_snapshot.revalidate(&repository)?;
        if local_oid(&repository)? != request.observation.head
            || Self::conflict_configuration(&root)? != request.observation.configuration
            || integration_conflict_digest(
                &mut repository,
                request.observation.window_number,
                request.observation.ordinal,
            )? != request.observation.fingerprint
            || !recorded_merge_index_matches(
                &repository,
                inspection.local_parent,
                inspection.incoming_parent,
            )?
            || !exact_resolution_merge_metadata(&mut repository, inspection.incoming_parent)?
            // Re-read the complete index/status/file preflight while the
            // repository lease is held, before durable intent or any write.
            || resolution_preflight(&repository, &request.resolutions)? != preflight_digest
        {
            return Ok(merge::ResolveSynchronizationOutcome::StaleObservation);
        }
        let effective_identity = repository.signature().ok().and_then(|signature| {
            let name = signature.name().unwrap_or_default();
            let email = signature.email().unwrap_or_default();
            (!name.is_empty() && !email.is_empty()).then(|| (name.to_owned(), email.to_owned()))
        });
        // A caller confirmation is evidence only when no effective repository
        // identity exists. Supplying one beside a valid signature must not
        // alter attempt input, durable bindings, or retry semantics.
        let confirmed_identity = effective_identity
            .is_none()
            .then(|| {
                request.identity.as_ref().filter(|identity| {
                    identity.expected_configuration == request.observation.configuration
                })
            })
            .flatten();
        let identity = effective_identity.or_else(|| {
            confirmed_identity.map(|identity| {
                (
                    identity.identity.name.clone(),
                    identity.identity.email.clone(),
                )
            })
        });
        let Some(identity) = identity.filter(|(name, email)| !name.is_empty() && !email.is_empty())
        else {
            return Ok(merge::ResolveSynchronizationOutcome::IdentityRequired);
        };
        let signature = git2::Signature::new(&identity.0, &identity.1, &git2::Time::new(0, 0))
            .map_err(|_| SynchronizationError::RecoveryRequired)?;
        #[cfg(any(unix, windows))]
        resolution_signer_digest(&signature)?;
        let mut input = blake3::Hasher::new();
        input.update(b"manyhands-resolution-v1\0");
        if let Some(identity) = confirmed_identity {
            input.update(identity.confirmation_id.to_string().as_bytes());
            input.update(identity.identity.name.as_bytes());
            input.update(identity.identity.email.as_bytes());
        }
        let mut path_intents = Vec::new();
        for path in &inspection.paths {
            let (token, result) = supplied
                .get(&path.token.ordinal)
                .expect("validated complete tokens");
            let expected_digest = conflict_token_digest(token);
            let relative = Path::new(
                std::str::from_utf8(&token.path)
                    .map_err(|_| SynchronizationError::RecoveryRequired)?,
            );
            let prewrite_digest = conflict_worktree_digest(
                preflight_repository
                    .workdir()
                    .ok_or(SynchronizationError::RecoveryRequired)?,
                relative,
            )?;
            input.update(&token.ordinal.to_be_bytes());
            input.update(&token.path);
            input.update(&expected_digest);
            input.update(result.bytes());
            path_intents.push(state::ResolutionPathIntent {
                ordinal: token.ordinal,
                path_digest: *blake3::hash(&token.path).as_bytes(),
                expected_digest,
                result_digest: *blake3::hash(result.bytes()).as_bytes(),
                prewrite_digest,
                base_blob_oid: token.base,
                local_blob_oid: token.local,
                incoming_blob_oid: token.incoming,
                mode: 0o100644,
            });
        }
        let identity_confirmation_id = confirmed_identity.map(|identity| identity.confirmation_id);
        if let Some(identity) = confirmed_identity {
            let mut identity_digest = blake3::Hasher::new();
            identity_digest.update(b"manyhands-resolution-identity-v1\0");
            identity_digest.update(identity.identity.name.as_bytes());
            identity_digest.update(identity.identity.email.as_bytes());
            self.prepare_synchronization_identity_confirmation(
                &root,
                &owner,
                &state::IdentityConfirmationIntent {
                    confirmation_id: identity.confirmation_id,
                    input_digest: *identity_digest.finalize().as_bytes(),
                    configuration_digest: request.observation.configuration,
                },
            )?;
            self.begin_synchronization_identity_confirmation_effect(
                &root,
                &owner,
                identity.confirmation_id,
            )?;
            self.observe_synchronization_identity_confirmation_effect(
                &root,
                &owner,
                identity.confirmation_id,
                request.observation.configuration,
            )?;
        }
        self.prepare_synchronization_resolution_attempt_in_window(
            &root,
            &owner,
            request.observation.window_number,
            &state::ResolutionAttemptIntent {
                attempt_id: request.attempt_id,
                step_ordinal: request.observation.ordinal,
                observation_digest: request.observation.fingerprint,
                input_digest: *input.finalize().as_bytes(),
                preflight_digest,
                identity_confirmation_id,
            },
            &path_intents,
        )?;
        self.begin_synchronization_resolution_path_effects(&root, &owner, request.attempt_id)?;
        let durable_paths = state::with_transaction(self, &root, |tx, id| {
            let record = state::read_operation(tx, id, request.synchronization_id)?
                .ok_or_else(state::recovery_required)?;
            state::resolution_paths_for_attempt(tx, &record, request.attempt_id)
        })?;
        if durable_paths.len() != inspection.paths.len() {
            return Ok(merge::ResolveSynchronizationOutcome::RecoveryRequired);
        }
        let mut index_lock =
            ResolutionIndexLock::acquire(&repository, self, &root, &owner, request.attempt_id)?;
        #[cfg(any(unix, windows))]
        {
            index_lock.metadata_matches(false)?;
            ref_snapshot.revalidate(&repository)?;
            refuse_ambiguous_resolution_backend_locks(&repository)?;
            index_lock.bind_ref_baseline(
                RefLogContext {
                    service: self,
                    root: &root,
                    owner: &owner,
                    request: &request,
                    input: *input.finalize().as_bytes(),
                },
                &ref_snapshot,
                &signature,
            )?;
        }
        // Git's index.lock now excludes external index writers while every
        // mutable input is checked one final time and through ref application.
        if !recorded_merge_index_matches(
            &repository,
            inspection.local_parent,
            inspection.incoming_parent,
        )? || !only_resolution_paths_are_dirty(&repository, &request.resolutions)?
            || !exact_resolution_merge_metadata(&mut repository, inspection.incoming_parent)?
            || resolution_preflight(&repository, &request.resolutions)? != preflight_digest
        {
            return Ok(merge::ResolveSynchronizationOutcome::StaleObservation);
        }

        let workdir = repository
            .workdir()
            .ok_or(SynchronizationError::RecoveryRequired)?;
        // Reject every third/unsafe image before completing any remaining
        // write in a multi-path attempt; recheck each member at its effect.
        for (path, durable) in inspection.paths.iter().zip(&durable_paths) {
            let (token, result) = supplied
                .get(&path.token.ordinal)
                .expect("validated complete tokens");
            resolution_path_needs_write(workdir, token, result.bytes(), durable)?;
        }
        let mut index = index_lock.authoritative_index()?;
        for (path, durable) in inspection.paths.iter().zip(&durable_paths) {
            let (token, result) = supplied
                .get(&path.token.ordinal)
                .expect("validated complete tokens");
            let relative = std::path::Path::new(
                std::str::from_utf8(&token.path)
                    .map_err(|_| SynchronizationError::RecoveryRequired)?,
            );
            let needs_write = resolution_path_needs_write(workdir, token, result.bytes(), durable)?;
            if needs_write {
                write_owned_document_if_prewrite_digest(
                    workdir,
                    relative,
                    result.bytes(),
                    durable.prewrite_digest,
                    RepositoryOperation::RepositorySnapshot,
                    &root,
                )?;
            }
            if !durable.applied {
                if owned_result_digest(workdir, relative)? != durable.result_digest {
                    return Err(SynchronizationError::ExternalChange);
                }
                // Equality proves only the bound path image, not ownership of
                // its inode or a private artifact. The fenced immutable attempt
                // authorizes observing it without another replacement.
                self.observe_synchronization_resolution_path_effect(
                    &root,
                    &owner,
                    request.attempt_id,
                    token.ordinal,
                )?;
                if needs_write {
                    self.check_failure(
                        FailurePoint::ResolutionAfterPathWrite,
                        RepositoryOperation::RepositorySnapshot,
                        &root,
                    )?;
                }
            }
            let blob = repository
                .blob(result.bytes())
                .map_err(|_| SynchronizationError::RecoveryRequired)?;
            for stage in 1..=3 {
                index
                    .remove(relative, stage)
                    .map_err(|_| SynchronizationError::RecoveryRequired)?;
            }
            index
                .add(&git2::IndexEntry {
                    ctime: git2::IndexTime::new(0, 0),
                    mtime: git2::IndexTime::new(0, 0),
                    dev: 0,
                    ino: 0,
                    mode: 0o100644,
                    uid: 0,
                    gid: 0,
                    file_size: result.bytes().len() as u32,
                    id: blob,
                    flags: 0,
                    flags_extended: 0,
                    path: token.path.clone(),
                })
                .map_err(|_| SynchronizationError::RecoveryRequired)?;
        }
        if index.has_conflicts() {
            return Ok(merge::ResolveSynchronizationOutcome::RecoveryRequired);
        }
        let tree_oid = index
            .write_tree_to(&repository)
            .map_err(|_| SynchronizationError::RecoveryRequired)?;
        let head = repository
            .head()
            .and_then(|value| value.peel_to_commit())
            .map_err(|_| SynchronizationError::RecoveryRequired)?;
        let incoming = repository
            .find_commit(inspection.incoming_parent)
            .map_err(|_| SynchronizationError::RecoveryRequired)?;
        if head.id() != inspection.local_parent {
            return Ok(merge::ResolveSynchronizationOutcome::StaleObservation);
        }
        let tree = repository
            .find_tree(tree_oid)
            .map_err(|_| SynchronizationError::RecoveryRequired)?;
        // A recorded retry must recreate exactly the same detached candidate.
        // The fixed timestamp is part of that durable candidate contract.
        let candidate = repository
            .commit(
                None,
                &signature,
                &signature,
                &match &inspection.target {
                    SynchronizationTarget::Primary => "Resolve synchronization primary".to_owned(),
                    SynchronizationTarget::Context { kind, item_id } => format!(
                        "Resolve synchronization {} {item_id}",
                        authoring_kind_segment(kind)
                    ),
                },
                &tree,
                &[&head, &incoming],
            )
            .map_err(|_| SynchronizationError::RecoveryRequired)?;
        #[cfg(any(unix, windows))]
        sync_resolution_candidate(&repository, candidate, &request.resolutions)?;
        self.prepare_synchronization_resolution_candidate(
            &root,
            &owner,
            request.attempt_id,
            candidate,
        )?;
        #[cfg(any(unix, windows))]
        let ref_proof = index_lock.bind_ref_transition(
            RefLogContext {
                service: self,
                root: &root,
                owner: &owner,
                request: &request,
                input: *input.finalize().as_bytes(),
            },
            candidate,
            &ref_snapshot,
            &signature,
        )?;
        self.check_failure(
            FailurePoint::ResolutionAfterCandidatePrepared,
            RepositoryOperation::RepositorySnapshot,
            &root,
        )?;
        let head_ref = repository
            .head()
            .map_err(|_| SynchronizationError::RecoveryRequired)?;
        let Some(name) = head_ref
            .symbolic_target()
            .or_else(|| head_ref.name())
            .map(str::to_owned)
        else {
            return Ok(merge::ResolveSynchronizationOutcome::RecoveryRequired);
        };
        index_lock.persist(
            &mut index,
            &repository,
            self,
            &root,
            &owner,
            request.attempt_id,
        )?;
        #[cfg(test)]
        run_resolution_index_effect_hook(
            repository
                .workdir()
                .ok_or(SynchronizationError::RecoveryRequired)?,
        );
        // The worktree already contains every bound result and the merge's
        // non-conflicting entries. Checkout would rewrite caller paths again.
        // Reload the installed index, then validate without materialization.
        index_lock.persisted_images_match()?;
        let mut installed_index = repository
            .index()
            .map_err(|_| SynchronizationError::RecoveryRequired)?;
        installed_index
            .read(true)
            .map_err(|_| SynchronizationError::RecoveryRequired)?;
        for (token, bytes) in &request.resolutions {
            let relative = Path::new(
                std::str::from_utf8(&token.path)
                    .map_err(|_| SynchronizationError::ExternalChange)?,
            );
            if owned_result_digest(workdir, relative)? != *blake3::hash(bytes.bytes()).as_bytes() {
                return Err(SynchronizationError::ExternalChange);
            }
        }
        let mut options = resolution_status_options();
        if repository
            .statuses(Some(&mut options))
            .map_err(|_| SynchronizationError::RecoveryRequired)?
            .iter()
            .any(|entry| {
                entry.status().intersects(
                    git2::Status::WT_NEW
                        | git2::Status::WT_MODIFIED
                        | git2::Status::WT_DELETED
                        | git2::Status::WT_TYPECHANGE
                        | git2::Status::WT_RENAMED
                        | git2::Status::CONFLICTED,
                )
            })
            || Self::conflict_configuration(&root)? != request.observation.configuration
        {
            return Err(SynchronizationError::ExternalChange);
        }
        #[cfg(any(unix, windows))]
        index_lock.metadata_matches(false)?;
        #[cfg(any(unix, windows))]
        refuse_ambiguous_resolution_backend_locks(&repository)?;
        #[cfg(any(unix, windows))]
        {
            if name != ref_snapshot.branch {
                return Err(SynchronizationError::RecoveryRequired);
            }
            apply_resolution_ref(
                &repository,
                RefLogContext {
                    service: self,
                    root: &root,
                    owner: &owner,
                    request: &request,
                    input: *input.finalize().as_bytes(),
                },
                candidate,
                &ref_snapshot,
                &ref_proof,
            )?;
        }
        #[cfg(not(any(unix, windows)))]
        {
            let _ = name;
            return Err(SynchronizationError::RecoveryRequired);
        }
        index_lock.persisted_images_match()?;
        drop(head_ref);
        self.check_failure(
            FailurePoint::ResolutionAfterRefTransition,
            RepositoryOperation::RepositorySnapshot,
            &root,
        )?;
        drop(lease);
        #[cfg(test)]
        run_resolution_index_hook(&RESOLUTION_REF_REFRESH_HOOK, &root);
        #[cfg(any(unix, windows))]
        {
            ref_snapshot = RefLogSnapshot::read(&repository)?;
        }
        let _lease = repository_lease(&repository, &root, RepositoryOperation::RepositorySnapshot)?;
        #[cfg(any(unix, windows))]
        {
            ref_snapshot.validate_final_proof(&repository, &ref_proof, candidate)?;
        }
        index_lock.persisted_images_match()?;
        #[cfg(any(unix, windows))]
        index_lock.metadata_matches(false)?;
        if foreign_resolution_metadata_present(&repository)
            || Self::conflict_configuration(&root)? != request.observation.configuration
        {
            return Err(SynchronizationError::ExternalChange);
        }
        for (token, bytes) in &request.resolutions {
            let relative = Path::new(
                std::str::from_utf8(&token.path)
                    .map_err(|_| SynchronizationError::ExternalChange)?,
            );
            if owned_result_digest(workdir, relative)? != *blake3::hash(bytes.bytes()).as_bytes() {
                return Err(SynchronizationError::ExternalChange);
            }
        }
        let mut options = resolution_status_options();
        if repository
            .statuses(Some(&mut options))
            .map_err(|_| SynchronizationError::RecoveryRequired)?
            .iter()
            .any(|entry| {
                entry.status().intersects(
                    git2::Status::WT_NEW
                        | git2::Status::WT_MODIFIED
                        | git2::Status::WT_DELETED
                        | git2::Status::WT_TYPECHANGE
                        | git2::Status::WT_RENAMED
                        | git2::Status::CONFLICTED,
                )
            })
        {
            return Err(SynchronizationError::ExternalChange);
        }
        let observed = local_oid(&repository)?;
        let observed_tree = repository
            .head()
            .and_then(|value| value.peel_to_commit())
            .and_then(|commit| commit.tree())
            .map_err(|_| SynchronizationError::RecoveryRequired)?
            .id();
        if observed != candidate || observed_tree != tree_oid {
            return Ok(merge::ResolveSynchronizationOutcome::RecoveryRequired);
        }
        drop(tree);
        drop(incoming);
        drop(head);
        #[cfg(any(unix, windows))]
        refuse_ambiguous_resolution_backend_locks(&repository)?;
        #[cfg(any(unix, windows))]
        {
            ref_snapshot.storage_barrier(&repository)?;
            ref_snapshot.validate_final_proof(&repository, &ref_proof, candidate)?;
        }
        self.advance_synchronization_resolution_ref_effect(
            &root,
            &owner,
            request.attempt_id,
            "observed",
        )?;
        self.observe_synchronization_resolution_checkpoint(
            &root,
            &owner,
            request.attempt_id,
            candidate,
            observed_tree,
        )?;
        self.check_failure(
            FailurePoint::ResolutionAfterCheckpointObservation,
            RepositoryOperation::RepositorySnapshot,
            &root,
        )?;
        // Metadata retirement is deliberately after the durable checkpoint. If
        // this fails, restart reconciliation sees the exact candidate instead
        // of manufacturing another merge.
        self.check_failure(
            FailurePoint::ResolutionBeforeMetadataRetirement,
            RepositoryOperation::RepositorySnapshot,
            &root,
        )?;
        if !exact_resolution_merge_metadata(&mut repository, inspection.incoming_parent)? {
            return Ok(merge::ResolveSynchronizationOutcome::RecoveryRequired);
        }
        index_lock.retire_metadata()?;
        index_lock.retire(self, &root, &owner, request.attempt_id)?;
        self.check_failure(
            FailurePoint::ResolutionAfterIndexLockRetirement,
            RepositoryOperation::RepositorySnapshot,
            &root,
        )?;
        self.check_failure(
            FailurePoint::ResolutionAfterMetadataCleanup,
            RepositoryOperation::RepositorySnapshot,
            &root,
        )?;
        self.finalize_synchronization_resolution(&root, &owner, request.attempt_id, candidate)?;
        Ok(
            merge::ResolveSynchronizationOutcome::LocalCheckpointComplete {
                commit_oid: candidate,
            },
        )
    }

    /// Synchronize one existing clean target; every publication is independently
    /// observed, and an authoritative replay performs only its index handoff.
    pub fn synchronize_remote<P: SessionCredentialProvider>(
        &self,
        mut request: SynchronizeRemoteRequest,
        session: &mut SessionCredentials<P>,
    ) -> Result<SynchronizationResult, SynchronizationError> {
        let (_, root) =
            canonical_repository_root(&request.root, RepositoryOperation::RepositorySnapshot)?;
        request.root = root.clone();
        let existing = state::with_transaction(self, &root, |tx, id| {
            let other_root: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM remote_operation_records WHERE operation_ulid=?1 AND repository_id!=?2)",rusqlite::params![request.operation_id.to_string(),id],|row|row.get(0)).map_err(|_|state::recovery_required())?;
            if other_root {
                return Err(identity_error());
            }
            state::read_operation(tx, id, request.operation_id)
        })?;
        // Historical identity is independent of current publication settings.
        // Check it before every replay/restart or live local-only selection.
        if let Some(record) = &existing {
            validate_record_target(&record.target, &request.target)?;
        }
        if let Some(record) = &existing
            && record.authority.is_some()
        {
            let RemoteReservationOutcome::Replay(inspection) =
                self.reserve_remote_operation(&root, request.operation_id, &record.target)?
            else {
                return Err(SynchronizationError::RecoveryRequired);
            };
            let outcome = authority_outcome(
                &request.target,
                inspection
                    .authority()
                    .ok_or(SynchronizationError::RecoveryRequired)?,
            );
            return Ok(if inspection.index_pending() {
                self.synchronization_refresh(&request, Some(&record.target), outcome)
            } else {
                SynchronizationResult::Complete(outcome)
            });
        }
        if let Some(record) = &existing
            && record.phase == RemoteOperationPhase::Cancelled
        {
            self.reserve_remote_operation(&root, request.operation_id, &record.target)?;
            return Err(SynchronizationError::Interrupted);
        }
        if let Some(record) = &existing
            && !request.restart
        {
            self.reserve_remote_operation(&root, request.operation_id, &record.target)?;
            return Err(SynchronizationError::RecoveryRequired);
        }
        if existing.is_none()
            && let Some(oid) = self.local_synchronization_replay(&request)?
        {
            return Ok(self.synchronization_refresh(
                &request,
                None,
                SynchronizationOutcome::PublishPending {
                    target: request.target.clone(),
                    local_oid: oid,
                    reason: PublishPendingReason::NoPublicationRemote,
                },
            ));
        }
        // A local child is authority for its recorded pass, not for fresh
        // transport. Reconcile it before endpoint discovery or clean preflight;
        // an identical pending conflict therefore works entirely offline.
        let mut local_owner = None;
        let mut local_evidence = None;
        let mut locally_reconciled = None;
        if let Some(record) = &existing {
            let has_child = state::with_transaction(self, &root, |tx, _| {
                let window = state::latest_integration_window(tx, record)?;
                Ok(window.intent.is_some()
                    || state::integration_step_in_window(tx, record.id, window.number, 0)?
                        .is_some()
                    || state::integration_step_in_window(tx, record.id, window.number, 1)?
                        .is_some())
            })?;
            if has_child {
                let owner = match self.restart_remote_synchronization(
                    &root,
                    request.operation_id,
                    &record.target,
                )? {
                    RemoteReservationOutcome::Reserved(owner) => owner,
                    RemoteReservationOutcome::Busy => return Err(SynchronizationError::Busy),
                    RemoteReservationOutcome::Replay(record)
                        if record.phase() == RemoteOperationPhase::Cancelled =>
                    {
                        return Err(SynchronizationError::Interrupted);
                    }
                    _ => return Err(SynchronizationError::RecoveryRequired),
                };
                let primary = record
                    .target
                    .primary_ref()
                    .remote_ref()
                    .strip_prefix("refs/heads/")
                    .ok_or(SynchronizationError::RecoveryRequired)?;
                let mut evidence = record.sync_evidence.clone();
                locally_reconciled = reconcile_pending_candidate(
                    self,
                    &root,
                    primary,
                    &request.target,
                    &owner,
                    &mut evidence,
                )?;
                local_owner = Some(owner);
                local_evidence = Some(evidence);
            }
        }
        let ConfigurationInspection::Valid(config) = read_configuration(&root)? else {
            return Err(SynchronizationError::RecoveryRequired);
        };
        let Some(remote_name) = config.publication_remote.as_ref() else {
            if existing.is_some() {
                return Err(SynchronizationError::RecoveryRequired);
            }
            let oid = self.bind_local_synchronization(&request, &config.primary_branch)?;
            return Ok(self.synchronization_refresh(
                &request,
                None,
                SynchronizationOutcome::PublishPending {
                    target: request.target.clone(),
                    local_oid: oid,
                    reason: PublishPendingReason::NoPublicationRemote,
                },
            ));
        };
        let plan = RemoteRefPlan::from_configuration(remote_name, &config.primary_branch)
            .map_err(|_| SynchronizationError::RecoveryRequired)?;
        let target = request.target.operation_target(&plan);
        // Inspect same-ID rows before reconfiguring or preflighting mutable Git.
        // The reservation controller validates root, target and local ID coexistence.
        if existing.is_none() {
            self.inspect_synchronization_local(&root, &config.primary_branch, &request.target)?;
        }
        // Freeze exactly the snapshot that establishes this generation. A same-ID
        // restart reconciles persisted endpoint identity BEFORE receiving ownership.
        let configuration = self.observation_configuration(&root, &plan)?;
        state::with_transaction(self, &root, |tx, id| {
            state::configure_endpoints(tx, id, &plan, &configuration.endpoint_digest())
        })?;
        let mut prior = None;
        let reservation = if let Some(owner) = local_owner {
            let record = existing
                .as_ref()
                .ok_or(SynchronizationError::RecoveryRequired)?;
            if record.target != target {
                return Err(SynchronizationError::RecoveryRequired);
            }
            prior = Some(record.clone().into());
            RemoteReservationOutcome::Reserved(owner)
        } else {
            self.reserve_remote_operation(&root, request.operation_id, &target)?
        };
        let reservation = match reservation {
            RemoteReservationOutcome::Replay(record) => {
                if let Some(authority) = record.authority() {
                    let outcome = authority_outcome(&request.target, authority);
                    return Ok(if record.index_pending() {
                        self.synchronization_refresh(&request, Some(&target), outcome)
                    } else {
                        SynchronizationResult::Complete(outcome)
                    });
                }
                if record.phase() == RemoteOperationPhase::Cancelled {
                    return Err(SynchronizationError::Interrupted);
                }
                if !request.restart {
                    return Err(SynchronizationError::RecoveryRequired);
                }
                prior = Some(record);
                self.restart_remote_synchronization(&root, request.operation_id, &target)
                    .map_err(|error| {
                        if error.kind == RepositoryErrorKind::RecoveryRequired {
                            SynchronizationError::RecoveryRequired
                        } else {
                            error.into()
                        }
                    })?
            }
            other => other,
        };
        let owner = match reservation {
            RemoteReservationOutcome::Reserved(owner) => owner,
            RemoteReservationOutcome::Busy => return Err(SynchronizationError::Busy),
            RemoteReservationOutcome::PollYielding => {
                return Err(SynchronizationError::PollYielding);
            }
            RemoteReservationOutcome::Replay(_) => {
                return Err(SynchronizationError::RecoveryRequired);
            }
        };
        if !self.synchronization_configuration_matches(&root, &plan, &configuration)? {
            return Err(SynchronizationError::ExternalChange);
        }
        let mut evidence = local_evidence.unwrap_or_else(|| {
            prior
                .as_ref()
                .map(|r: &reservation::RemoteOperationInspection| r.sync_evidence().clone())
                .unwrap_or_default()
        });
        let mut pending_candidate = if locally_reconciled.is_some() {
            locally_reconciled
        } else if prior.is_some() {
            reconcile_pending_candidate(
                self,
                &root,
                &config.primary_branch,
                &request.target,
                &owner,
                &mut evidence,
            )?
        } else {
            None
        };
        if prior.is_some() {
            let (window, unfinished) = state::with_transaction(self, &root, |tx, id| {
                let record = reservation::owned(self, tx, id, &owner)?;
                let window = state::latest_integration_window(tx, &record)?;
                let first = state::integration_step_in_window(tx, record.id, window.number, 0)?;
                let primary_ordinal = u8::from(record.target.context_ref().is_some());
                let last = state::integration_step_in_window(
                    tx,
                    record.id,
                    window.number,
                    primary_ordinal,
                )?;
                let started = window.intent.is_some() || first.is_some() || last.is_some();
                let unfinished = started
                    && last
                        .as_ref()
                        .is_none_or(|step| step.phase != state::IntegrationStepPhase::Applied);
                Ok((window, unfinished))
            })?;
            if unfinished {
                // Complete only the original frozen pass before any new Fetch.
                // In particular, an applied context never gets replayed when
                // only its primary slot remains prepared or absent.
                let frozen_primary = window
                    .intent
                    .as_ref()
                    .map(|pass| pass.primary_oid)
                    .or(evidence.primary_tracking_oid)
                    .ok_or(SynchronizationError::RecoveryRequired)?;
                let frozen_context = if let Some(pass) = &window.intent {
                    pass.context_oid
                } else {
                    state::with_transaction(self, &root, |tx, id| {
                        let record = reservation::owned(self, tx, id, &owner)?;
                        Ok(state::integration_step_in_window(tx, record.id, 0, 0)?
                            .filter(|step| step.intent.stage == merge::IntegrationStage::Context)
                            .map(|step| step.intent.incoming_oid))
                    })?
                };
                let selected = target_ref(&plan, &request.target);
                let primary_tracking = window
                    .intent
                    .as_ref()
                    .map(|pass| pass.primary_oid)
                    .or(evidence.primary_tracking_oid);
                let selected_tracking = if matches!(request.target, SynchronizationTarget::Primary)
                {
                    primary_tracking
                } else {
                    frozen_context
                };
                let merged = integrate_divergence(
                    self,
                    DivergenceInputs {
                        root: &root,
                        primary_branch: &config.primary_branch,
                        target: &request.target,
                        request: &request,
                        owner: &owner,
                        plan: &plan,
                        configuration: &configuration,
                        selected: &selected,
                        primary_tracking,
                        selected_tracking,
                        context: frozen_context,
                        primary: frozen_primary,
                    },
                )?;
                evidence.local_oid = Some(merged);
                pending_candidate = reconcile_pending_candidate(
                    self,
                    &root,
                    &config.primary_branch,
                    &request.target,
                    &owner,
                    &mut evidence,
                )?;
            }
        }
        let initial = self
            .inspect_synchronization_local(&root, &config.primary_branch, &request.target)
            .map_err(|error| {
                if prior.as_ref().is_some_and(|record| {
                    matches!(
                        record.sync_checkpoint(),
                        Some(
                            Checkpoint::LocalPrepared
                                | Checkpoint::LocalFastForwarded
                                | Checkpoint::PushPrepared
                                | Checkpoint::PushReturned
                                | Checkpoint::PushVerified
                        )
                    )
                }) {
                    SynchronizationError::RecoveryRequired
                } else {
                    error
                }
            })?;
        if prior.is_none() {
            evidence = Evidence {
                expected_oid: Some(initial),
                local_oid: Some(initial),
                ..Evidence::default()
            };
        }
        let candidate_reconciled = pending_candidate.is_some();
        if prior.is_none() {
            decision(self.checkpoint_synchronization(
                &root,
                &owner,
                Checkpoint::FetchPrepared,
                &evidence,
            )?)?;
        }
        self.synchronization_point(&root, &owner, RemoteOperationSafePoint::BeforeFetch)?;
        let transport_request = |direction| VerifySshTransportRequest {
            root: root.clone(),
            direction,
            approval: request.approval.clone(),
        };
        let before = self.with_synchronization_remote(
            transport_request(SshDirection::Fetch),
            session,
            &owner,
            &plan,
            &configuration,
            |r| r.fresh_advertisement(),
        )?;
        self.synchronization_boundary(&root, &owner)?;
        self.inspect_synchronization_local(&root, &config.primary_branch, &request.target)?;
        self.with_synchronization_remote(
            transport_request(SshDirection::Fetch),
            session,
            &owner,
            &plan,
            &configuration,
            |r| r.fetch_exact(&plan, &request.target),
        )?;
        self.synchronization_boundary(&root, &owner)?;
        self.inspect_synchronization_local(&root, &config.primary_branch, &request.target)?;
        let after = self.with_synchronization_remote(
            transport_request(SshDirection::Fetch),
            session,
            &owner,
            &plan,
            &configuration,
            |r| r.fresh_advertisement(),
        )?;
        self.synchronization_boundary(&root, &owner)?;
        self.inspect_synchronization_local(&root, &config.primary_branch, &request.target)?;
        let selected = target_ref(&plan, &request.target);
        let primary = advertised_oid(&after, plan.primary().remote_ref())?;
        let context = if matches!(request.target, SynchronizationTarget::Context { .. }) {
            advertised_oid(&after, selected.remote_ref())?
        } else {
            None
        };
        let repository =
            git2::Repository::open(&root).map_err(|_| SynchronizationError::RecoveryRequired)?;
        let mut changed = false;
        for relevant in [plan.primary(), &selected] {
            let final_oid = advertised_oid(&after, relevant.remote_ref())?;
            if advertised_oid(&before, relevant.remote_ref())? != final_oid
                || final_oid.is_some_and(|oid| {
                    repository.refname_to_id(relevant.tracking_ref()).ok() != Some(oid)
                })
            {
                changed = true;
            }
        }
        let observations = after
            .iter()
            .filter_map(|(name, oid)| {
                RemoteRefObservation::from_advertisement(
                    &plan,
                    name,
                    *oid,
                    plan.tracking_ref_for(name)
                        .and_then(|name| repository.refname_to_id(&name).ok()),
                )
            })
            .collect::<Vec<_>>();
        if self.observation_configuration(&root, &plan)? != configuration {
            return Err(SynchronizationError::ExternalChange);
        }
        decision(reservation::commit_observation_batch(
            self,
            &root,
            &owner,
            &plan,
            &observations,
            time::OffsetDateTime::now_utc().unix_timestamp().max(0),
        )?)?;
        self.synchronization_point(&root, &owner, RemoteOperationSafePoint::AfterFetch)?;
        if changed {
            return Err(SynchronizationError::ExternalChange);
        }
        evidence.primary_tracking_oid = primary;
        evidence.tracking_oid = if matches!(request.target, SynchronizationTarget::Primary) {
            primary
        } else {
            context
        };
        let publication = match &request.target {
            SynchronizationTarget::Primary => RemotePublicationEvidence::HistoryUnknown,
            SynchronizationTarget::Context { kind, item_id } => self
                .remote_snapshot(&root)?
                .publication_evidence_for(*kind, item_id),
        };
        // Fetch and Push history are direction-specific. A verified publication
        // at this endpoint must not be recreated after deletion, even when its
        // Fetch advertisement has always been absent (distinct pushurl).
        if primary.is_none() {
            return Err(SynchronizationError::PrimaryMissing);
        }
        let mut resumed = None;
        if let Some(candidate) = pending_candidate {
            finalize_reconciled_candidate(self, &root, &owner, candidate, &evidence)?;
            let record = state::with_transaction(self, &root, |tx, id| {
                state::read_operation(tx, id, request.operation_id)
            })?
            .ok_or(SynchronizationError::RecoveryRequired)?;
            evidence = record.sync_evidence;
            resumed = record.sync_checkpoint;
        }
        let push_absence_boundary =
            self.synchronization_push_absence_boundary(&root, &owner, prior.is_some())?;
        if let Some(boundary) = push_absence_boundary {
            let actual =
                self.inspect_synchronization_local(&root, &config.primary_branch, &request.target)?;
            let observed = self.synchronization_push_observation(
                &request,
                session,
                &owner,
                &plan,
                &configuration,
                actual,
            )?;
            if self.observation_configuration(&root, &plan)? != configuration {
                return Err(SynchronizationError::ExternalChange);
            }
            if observed.is_none() {
                return Err(boundary.error());
            }
        }
        if prior.is_some() && !candidate_reconciled {
            let actual =
                self.inspect_synchronization_local(&root, &config.primary_branch, &request.target)?;
            let push = self.synchronization_push_observation(
                &request,
                session,
                &owner,
                &plan,
                &configuration,
                actual,
            )?;
            let ancestor = push.is_some_and(|oid| {
                oid != actual && repository.graph_descendant_of(actual, oid).unwrap_or(false)
            });
            decision(
                self.reconcile_synchronization(&root, &owner, actual, actual, push, ancestor)?,
            )?;
            let record = state::with_transaction(self, &root, |tx, id| {
                state::read_operation(tx, id, request.operation_id)
            })?
            .ok_or(SynchronizationError::RecoveryRequired)?;
            evidence = record.sync_evidence;
            evidence.primary_tracking_oid = primary;
            evidence.tracking_oid = if matches!(request.target, SynchronizationTarget::Primary) {
                primary
            } else {
                context
            };
            resumed = record.sync_checkpoint;
        }
        // Preserve Cycle 05's fresh all-clean virtual plan. Divergence and
        // existing ordered-pass continuations use the child journal.
        let divergence = {
            let local =
                self.inspect_synchronization_local(&root, &config.primary_branch, &request.target)?;
            match graph_plan(
                &repository,
                &request.target,
                local,
                primary,
                context,
                publication,
            ) {
                Ok(_) => false,
                Err(SynchronizationError::MergeRequired { .. }) => true,
                Err(error) => return Err(error),
            }
        };
        let ordered_continuation = state::with_transaction(self, &root, |tx, id| {
            let record = reservation::owned(self, tx, id, &owner)?;
            let window = state::latest_integration_window(tx, &record)?;
            Ok(record.sync_evidence.push_oid.is_none()
                && (window.intent.is_some()
                    || state::integration_step_in_window(tx, record.id, window.number, 0)?
                        .is_some()
                    || state::integration_step_in_window(tx, record.id, window.number, 1)?
                        .is_some()))
        })?;
        if divergence || ordered_continuation {
            let local =
                self.inspect_synchronization_local(&root, &config.primary_branch, &request.target)?;
            let (window, batch, completed_oid) = state::with_transaction(self, &root, |tx, id| {
                let record = reservation::owned(self, tx, id, &owner)?;
                let window = state::latest_integration_window(tx, &record)?;
                let batch = tx.query_row(
                    "SELECT id FROM remote_observation_batches WHERE repository_id=?1 AND is_current=1",
                    [id],
                    |row| row.get::<_, i64>(0),
                ).map_err(|_| state::recovery_required())?;
                let ordinal = u8::from(record.target.context_ref().is_some());
                let completed_oid =
                    state::integration_step_in_window(tx, record.id, window.number, ordinal)?
                        .and_then(|step| step.result_oid);
                Ok((window, batch, completed_oid))
            })?;
            let intent = state::IntegrationWindowIntent {
                observation_batch_id: batch,
                local_oid: local,
                primary_oid: primary.ok_or(SynchronizationError::PrimaryMissing)?,
                context_oid: context,
            };
            let unchanged = window.intent.as_ref().is_some_and(|old| {
                old.primary_oid == intent.primary_oid
                    && old.context_oid == intent.context_oid
                    && completed_oid == Some(local)
            });
            // An unchanged, fully integrated pass has no new local effect.
            // Changed refs or a released descendant get a new immutable pass.
            if !unchanged || divergence {
                self.prepare_synchronization_window(
                    &root,
                    &owner,
                    window
                        .number
                        .checked_add(1)
                        .ok_or(SynchronizationError::RecoveryRequired)?,
                    &intent,
                )?;
                let merged = integrate_divergence(
                    self,
                    DivergenceInputs {
                        root: &root,
                        primary_branch: &config.primary_branch,
                        target: &request.target,
                        request: &request,
                        owner: &owner,
                        plan: &plan,
                        configuration: &configuration,
                        selected: &selected,
                        primary_tracking: evidence.primary_tracking_oid,
                        selected_tracking: evidence.tracking_oid,
                        context,
                        primary: primary.ok_or(SynchronizationError::PrimaryMissing)?,
                    },
                )?;
                evidence.local_oid = Some(merged);
                decision(self.checkpoint_synchronization_merge_applied(&root, &owner, &evidence)?)?;
            }
        } else {
            // Compute the entire virtual graph under one short lease before any write.
            {
                let _lease =
                    repository_lease(&repository, &root, RepositoryOperation::RepositorySnapshot)?;
                if self.observation_configuration(&root, &plan)? != configuration {
                    return Err(SynchronizationError::ExternalChange);
                }
                for (reference, oid) in [
                    (plan.primary(), primary),
                    (&selected, evidence.tracking_oid),
                ] {
                    if oid.is_some_and(|oid| {
                        repository.refname_to_id(reference.tracking_ref()).ok() != Some(oid)
                    }) {
                        return Err(SynchronizationError::ExternalChange);
                    }
                }
                let linked = local_target(&root, &config.primary_branch, &request.target)?;
                let local = local_oid(&linked)?;
                if evidence.expected_oid != Some(local)
                    && !matches!(
                        resumed,
                        Some(
                            Checkpoint::LocalFastForwarded
                                | Checkpoint::PushPrepared
                                | Checkpoint::PushVerified
                        )
                    )
                {
                    return Err(SynchronizationError::ExternalChange);
                }
                let integration = graph_plan(
                    &repository,
                    &request.target,
                    local,
                    primary,
                    context,
                    publication,
                )?;
                if resumed.is_some_and(|c| {
                    matches!(
                        c,
                        Checkpoint::LocalFastForwarded
                            | Checkpoint::PushPrepared
                            | Checkpoint::PushVerified
                    )
                }) && integration.final_oid != local
                {
                    return Err(SynchronizationError::RecoveryRequired);
                }
                evidence.local_oid = Some(integration.final_oid);
                if integration.local_update {
                    decision(self.checkpoint_synchronization(
                        &root,
                        &owner,
                        Checkpoint::LocalPrepared,
                        &evidence,
                    )?)?;
                    self.synchronization_point(
                        &root,
                        &owner,
                        RemoteOperationSafePoint::BeforeLocalUpdate,
                    )?;
                    // Reopen at the actual mutation boundary after durable intent.
                    let fresh = local_target(&root, &config.primary_branch, &request.target)?;
                    if local_oid(&fresh)? != local {
                        return Err(SynchronizationError::ExternalChange);
                    }
                    fast_forward(&fresh, selected.remote_ref(), local, integration.final_oid)?;
                    local_target(&root, &config.primary_branch, &request.target)
                        .map_err(|_| SynchronizationError::RecoveryRequired)?;
                    if local_oid(&fresh)? != integration.final_oid {
                        return Err(SynchronizationError::RecoveryRequired);
                    }
                    decision(self.checkpoint_synchronization(
                        &root,
                        &owner,
                        Checkpoint::LocalFastForwarded,
                        &evidence,
                    )?)?;
                    self.synchronization_point(
                        &root,
                        &owner,
                        RemoteOperationSafePoint::AfterLocalUpdate,
                    )?;
                }
            }
        }
        self.synchronization_boundary(&root, &owner)?;
        let candidate = evidence
            .local_oid
            .ok_or(SynchronizationError::RecoveryRequired)?;
        let push = self.synchronization_push_observation(
            &request,
            session,
            &owner,
            &plan,
            &configuration,
            candidate,
        )?;
        if self.inspect_synchronization_local(&root, &config.primary_branch, &request.target)?
            != candidate
        {
            return Err(SynchronizationError::ExternalChange);
        }
        if self.observation_configuration(&root, &plan)? != configuration {
            return Err(SynchronizationError::ExternalChange);
        }
        if push.is_none()
            && let Some(boundary) = push_absence_boundary
        {
            return Err(boundary.error());
        }
        evidence.push_oid = Some(candidate);
        evidence.push_advertised_oid = push;
        let authority = if push == Some(candidate) {
            if resumed != Some(Checkpoint::PushVerified) {
                decision(self.checkpoint_synchronization(
                    &root,
                    &owner,
                    Checkpoint::PushVerified,
                    &evidence,
                )?)?;
            }
            if prior.as_ref().is_some_and(|r| {
                r.sync_checkpoint().is_some() && r.sync_evidence().push_oid.is_some()
            }) {
                Authority::Published(candidate)
            } else {
                Authority::AlreadyCurrent(candidate)
            }
        } else {
            if resumed == Some(Checkpoint::PushVerified) {
                return Err(SynchronizationError::RecoveryRequired);
            }
            if push.is_some_and(|oid| {
                !repository
                    .graph_descendant_of(candidate, oid)
                    .unwrap_or(false)
            }) {
                return Err(SynchronizationError::PushRejected);
            }
            if resumed != Some(Checkpoint::PushPrepared) {
                decision(self.checkpoint_synchronization(
                    &root,
                    &owner,
                    Checkpoint::PushPrepared,
                    &evidence,
                )?)?;
            }
            self.synchronization_point(&root, &owner, RemoteOperationSafePoint::BeforePush)?;
            if self.inspect_synchronization_local(&root, &config.primary_branch, &request.target)?
                != candidate
            {
                return Err(SynchronizationError::ExternalChange);
            }
            self.with_synchronization_remote(
                transport_request(SshDirection::Push),
                session,
                &owner,
                &plan,
                &configuration,
                |r| r.push_exact(&plan, &request.target),
            )
            .map_err(unverified_push_error)?;
            decision(self.checkpoint_synchronization(
                &root,
                &owner,
                Checkpoint::PushReturned,
                &evidence,
            )?)?;
            self.synchronization_point(&root, &owner, RemoteOperationSafePoint::AfterPushReturn)?;
            let verified = self
                .with_synchronization_remote(
                    transport_request(SshDirection::Push),
                    session,
                    &owner,
                    &plan,
                    &configuration,
                    |r| r.fresh_advertisement(),
                )
                .map_err(unverified_push_error)?;
            self.synchronization_boundary(&root, &owner)?;
            if self.inspect_synchronization_local(&root, &config.primary_branch, &request.target)?
                != candidate
            {
                return Err(SynchronizationError::RecoveryRequired);
            }
            evidence.push_advertised_oid = advertised_oid(&verified, selected.remote_ref())?;
            if evidence.push_advertised_oid != Some(candidate) {
                return Err(SynchronizationError::RecoveryRequired);
            }
            if !self.synchronization_configuration_matches(&root, &plan, &configuration)? {
                return Err(SynchronizationError::RecoveryRequired);
            }
            decision(self.checkpoint_synchronization(
                &root,
                &owner,
                Checkpoint::PushVerified,
                &evidence,
            )?)?;
            Authority::Published(candidate)
        };
        self.synchronization_point(
            &root,
            &owner,
            RemoteOperationSafePoint::AfterPushVerification,
        )?;
        if !self.synchronization_configuration_matches(&root, &plan, &configuration)? {
            return Err(SynchronizationError::RecoveryRequired);
        }
        self.synchronization_boundary(&root, &owner)?;
        decision(self.classify_synchronization(&root, &owner, authority)?)?;
        Ok(self.synchronization_refresh(
            &request,
            Some(&target),
            authority_outcome(&request.target, authority),
        ))
    }
    /// Endpoint-qualified proof stays in the existing envelope, never in Fetch
    /// history. Endpoint/plan edits monotonically fence configuration generations.
    fn synchronization_push_absence_boundary(
        &self,
        root: &Path,
        owner: &RemoteReservation,
        inherited: bool,
    ) -> Result<Option<PushAbsenceBoundary>, SynchronizationError> {
        state::with_transaction(self, root, |tx, id| {
            let current = reservation::owned(self, tx, id, owner)?;
            let Some((kind, item_id)) = current.target.item() else { return Ok(None); };
            let mut query = tx.prepare("SELECT operation_ulid FROM remote_operation_records WHERE repository_id=?1 AND action='synchronize_context' AND kind=?2 AND item_id=?3").map_err(|_|state::recovery_required())?;
            let ids = query.query_map(rusqlite::params![id,authoring_kind_segment(&kind),item_id.to_string()],|row|row.get::<_,String>(0)).map_err(|_|state::recovery_required())?.collect::<Result<Vec<_>,_>>().map_err(|_|state::recovery_required())?;
            let mut verified = false;
            let mut ambiguous = false;
            let mut incompatible = false;
            for operation in ids {
                let operation = OperationId::parse(&operation).map_err(|_|state::recovery_required())?;
                if operation == owner.operation_id() && !inherited { continue; }
                let record = state::read_operation(tx,id,operation)?.ok_or_else(state::recovery_required)?;
                if record.generation > current.generation || record.target.local_branch() != current.target.local_branch() || record.target.context_ref().map(RemoteRefTarget::remote_ref) != current.target.context_ref().map(RemoteRefTarget::remote_ref) { return Err(state::recovery_required()); }
                let evidence = &record.sync_evidence;
                let proven = record.authority.is_some() || record.sync_checkpoint == Some(Checkpoint::PushVerified) && evidence.push_oid.is_some() && evidence.push_advertised_oid == evidence.push_oid;
                let intent = evidence.push_oid.is_some() && matches!(record.sync_checkpoint,Some(Checkpoint::PushPrepared | Checkpoint::PushReturned));
                if proven || intent {
                    if record.generation != current.generation || record.target != current.target { incompatible = true; }
                    else if proven { verified = true; }
                    else { ambiguous = true; }
                }
            }
            Ok(if verified { Some(PushAbsenceBoundary::Deleted) }
                else if ambiguous { Some(PushAbsenceBoundary::Ambiguous) }
                else if incompatible { Some(PushAbsenceBoundary::Unknown) }
                else { None })
        }).map_err(SynchronizationError::Repository)
    }

    fn synchronization_push_observation<P: SessionCredentialProvider>(
        &self,
        request: &SynchronizeRemoteRequest,
        session: &mut SessionCredentials<P>,
        owner: &RemoteReservation,
        plan: &RemoteRefPlan,
        configuration: &super::observation::ObservationConfiguration,
        candidate: git2::Oid,
    ) -> Result<Option<git2::Oid>, SynchronizationError> {
        let transport = VerifySshTransportRequest {
            root: request.root.clone(),
            direction: SshDirection::Push,
            approval: request.approval.clone(),
        };
        let advertised = self.with_synchronization_remote(
            transport.clone(),
            session,
            owner,
            plan,
            configuration,
            |r| r.fresh_advertisement(),
        )?;
        self.synchronization_boundary(&request.root, owner)?;
        self.inspect_synchronization_local(
            &request.root,
            plan.primary()
                .remote_ref()
                .strip_prefix("refs/heads/")
                .ok_or(SynchronizationError::RecoveryRequired)?,
            &request.target,
        )?;
        let selected = target_ref(plan, &request.target);
        let oid = advertised_oid(&advertised, selected.remote_ref())?;
        if let Some(oid) = oid
            && oid != candidate
        {
            let repository = git2::Repository::open(&request.root)
                .map_err(|_| SynchronizationError::RecoveryRequired)?;
            if repository.find_commit(oid).is_err() {
                let downloaded = self.with_synchronization_remote(
                    transport.clone(),
                    session,
                    owner,
                    plan,
                    configuration,
                    |r| r.download_push_target(plan, &request.target),
                )?;
                self.synchronization_boundary(&request.root, owner)?;
                if downloaded != Some(oid) || repository.find_commit(oid).is_err() {
                    return Err(SynchronizationError::ExternalChange);
                }
            }
            // Receive-pack proof after upload-pack object acquisition, never assume
            // that an upload-pack advertisement names the same current target.
            let fresh = self.with_synchronization_remote(
                transport,
                session,
                owner,
                plan,
                configuration,
                |r| r.fresh_advertisement(),
            )?;
            self.synchronization_boundary(&request.root, owner)?;
            if advertised_oid(&fresh, selected.remote_ref())? != Some(oid) {
                return Err(SynchronizationError::ExternalChange);
            }
        }
        Ok(oid)
    }
    fn bind_local_synchronization(
        &self,
        request: &SynchronizeRemoteRequest,
        primary: &str,
    ) -> Result<git2::Oid, SynchronizationError> {
        #[cfg(test)]
        tests::run_local_binding_hook(request.operation_id);
        let repository = git2::Repository::open(&request.root)
            .map_err(|_| SynchronizationError::RecoveryRequired)?;
        let _lease = repository_lease(
            &repository,
            &request.root,
            RepositoryOperation::RefreshRepository,
        )?;
        let oid = local_oid(&local_target(&request.root, primary, &request.target)?)?;
        let _guard = cache_write_guard(
            &self.registry_path,
            &request.root,
            RepositoryOperation::RefreshRepository,
        )?;
        let mut connection = open_registry(&self.registry_path, &mut |_| {})
            .map_err(|_| SynchronizationError::RecoveryRequired)?;
        crate::repository::recovery::begin_or_reconcile_local_synchronization(
            &mut connection,
            &request.root,
            RepositoryOperation::RefreshRepository,
            request.operation_id,
            &format!("{}/{oid}", local_refresh_identity(&request.target)),
        )
        .map_err(|_| SynchronizationError::RecoveryRequired)?;
        Ok(oid)
    }

    fn local_synchronization_replay(
        &self,
        request: &SynchronizeRemoteRequest,
    ) -> Result<Option<git2::Oid>, SynchronizationError> {
        let row = refresh_row(self, request)?;
        let Some((root, action, matcher, _)) = row else {
            return Ok(None);
        };
        if root != request.root.to_string_lossy() || action != "refresh" {
            return Err(identity_mismatch());
        }
        let Some((identity, oid)) = matcher.rsplit_once('/') else {
            return Err(identity_mismatch());
        };
        let oid = git2::Oid::from_str(oid).map_err(|_| identity_mismatch())?;
        if identity != local_refresh_identity(&request.target) {
            return Err(identity_mismatch());
        }
        Ok(Some(oid))
    }
    fn synchronization_refresh(
        &self,
        request: &SynchronizeRemoteRequest,
        target: Option<&RemoteOperationTarget>,
        outcome: SynchronizationOutcome,
    ) -> SynchronizationResult {
        let matcher = match &outcome {
            SynchronizationOutcome::PublishPending {
                target, local_oid, ..
            } => format!("{}/{local_oid}", local_refresh_identity(target)),
            _ => String::new(),
        };
        let completed = refresh_row(self, request).is_ok_and(|row| {
            row.is_some_and(|(root, action, stored, state)| {
                root == request.root.to_string_lossy()
                    && action == "refresh"
                    && stored == matcher
                    && state == "completed"
            })
        });
        let refreshed = completed
            || matches!(
                self.refresh_repository_target(
                    RefreshRepositoryRequest {
                        root: request.root.clone(),
                        operation_id: request.operation_id
                    },
                    &matcher
                ),
                Ok(RefreshOutcome::Refreshed { .. })
            );
        if refreshed
            && target.is_none_or(|target| {
                self.finish_synchronization_index(&request.root, request.operation_id, target)
                    .is_ok()
            })
        {
            SynchronizationResult::Complete(outcome)
        } else {
            SynchronizationResult::IndexPending(IndexPending::new(outcome))
        }
    }
}
fn authority_outcome(
    target: &SynchronizationTarget,
    authority: Authority,
) -> SynchronizationOutcome {
    match authority {
        Authority::Published(oid) => SynchronizationOutcome::Published {
            target: target.clone(),
            oid,
        },
        Authority::AlreadyCurrent(oid) => SynchronizationOutcome::AlreadyCurrent {
            target: target.clone(),
            oid,
        },
    }
}
fn fast_forward(
    repository: &git2::Repository,
    branch: &str,
    old: git2::Oid,
    new: git2::Oid,
) -> Result<(), SynchronizationError> {
    let commit = repository
        .find_commit(new)
        .map_err(|_| SynchronizationError::RecoveryRequired)?;
    // Lock/compare before effects. Locked git2 has no checkout baseline setter:
    // safe checkout must see the OLD HEAD, then the single ref commit follows.
    let mut transaction = repository
        .transaction()
        .map_err(|_| SynchronizationError::ExternalChange)?;
    transaction
        .lock_ref(branch)
        .map_err(|_| SynchronizationError::ExternalChange)?;
    if repository.refname_to_id(branch).ok() != Some(old)
        || repository
            .find_reference("HEAD")
            .ok()
            .and_then(|r| r.symbolic_target().map(str::to_owned))
            .as_deref()
            != Some(branch)
    {
        return Err(SynchronizationError::ExternalChange);
    }
    let mut checkout = git2::build::CheckoutBuilder::new();
    // SAFE alone defaults to overwriting ignored user entries in locked libgit2.
    checkout.safe().overwrite_ignored(false);
    repository
        .checkout_tree(commit.as_object(), Some(&mut checkout))
        .map_err(|_| SynchronizationError::RecoveryRequired)?;
    #[cfg(test)]
    super::observation_tests::checkpoint(RemoteOperationSafePoint::BeforeLocalMutation);
    if repository.refname_to_id(branch).ok() != Some(old)
        || repository
            .find_reference("HEAD")
            .ok()
            .and_then(|r| r.symbolic_target().map(str::to_owned))
            .as_deref()
            != Some(branch)
    {
        return Err(SynchronizationError::RecoveryRequired);
    }
    transaction
        .set_target(branch, new, None, "manyhands synchronization")
        .map_err(|_| SynchronizationError::RecoveryRequired)?;
    transaction
        .commit()
        .map_err(|_| SynchronizationError::RecoveryRequired)?;
    Ok(())
}
fn identity_mismatch() -> SynchronizationError {
    identity_error().into()
}
fn identity_error() -> RepositoryError {
    RepositoryError::new(
        RepositoryOperation::RepositorySnapshot,
        None,
        RepositoryErrorKind::OperationMismatch,
        "synchronization identity mismatch",
    )
}
fn local_refresh_identity(target: &SynchronizationTarget) -> String {
    match target {
        SynchronizationTarget::Primary => "synchronization-local-v1/primary".into(),
        SynchronizationTarget::Context { kind, item_id } => format!(
            "synchronization-local-v1/{}/{item_id}",
            authoring_kind_segment(kind)
        ),
    }
}
fn validate_record_target(
    record: &RemoteOperationTarget,
    target: &SynchronizationTarget,
) -> Result<(), SynchronizationError> {
    let valid = match target {
        SynchronizationTarget::Primary => {
            record.action() == RemoteOperationAction::SynchronizePrimary && record.item().is_none()
        }
        SynchronizationTarget::Context { kind, item_id } => {
            record.action() == RemoteOperationAction::SynchronizeContext
                && record.item() == Some((*kind, item_id))
        }
    };
    if valid {
        Ok(())
    } else {
        Err(identity_mismatch())
    }
}
type RefreshRow = (String, String, String, String);
fn refresh_row(
    service: &RepositoryService,
    request: &SynchronizeRemoteRequest,
) -> Result<Option<RefreshRow>, RepositoryError> {
    use rusqlite::OptionalExtension;
    state::with_transaction(service, &request.root, |tx, _| {
        tx.query_row("SELECT root_path,action,coalesce(target,''),state FROM operation_records WHERE operation_ulid=?1",[request.operation_id.to_string()],|row|Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?))).optional().map_err(|_|state::recovery_required())
    })
}
