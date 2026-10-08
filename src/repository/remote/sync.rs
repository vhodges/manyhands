//! Deliberate synchronization composes the owned state and scoped transport seams.
#![allow(clippy::result_large_err)]
#[cfg(test)]
#[path = "sync_tests.rs"]
mod tests;
use super::*;
use crate::repository::{keys::*, transport::*, *};
use state::{
    SynchronizationAuthority as Authority, SynchronizationCheckpoint as Checkpoint,
    SynchronizationEvidence as Evidence,
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
fn set_resolution_index_lock_hook(root: PathBuf, hook: impl FnOnce() + Send + 'static) {
    RESOLUTION_INDEX_LOCK_HOOK
        .get_or_init(|| std::sync::Mutex::new(Vec::new()))
        .lock()
        .expect("resolution index-lock hook")
        .push((root, Box::new(hook)));
}

#[cfg(test)]
fn set_resolution_index_persist_hook(root: PathBuf, hook: impl FnOnce() + Send + 'static) {
    RESOLUTION_INDEX_PERSIST_HOOK
        .get_or_init(|| std::sync::Mutex::new(Vec::new()))
        .lock()
        .expect("resolution index-persist hook")
        .push((root, Box::new(hook)));
}

#[cfg(test)]
fn set_resolution_index_scratch_hook(root: PathBuf, hook: impl FnOnce() + Send + 'static) {
    RESOLUTION_INDEX_SCRATCH_HOOK
        .get_or_init(|| std::sync::Mutex::new(Vec::new()))
        .lock()
        .expect("resolution index-scratch hook")
        .push((root, Box::new(hook)));
}

#[cfg(test)]
fn set_resolution_index_install_hook(root: PathBuf, hook: impl FnOnce() + Send + 'static) {
    RESOLUTION_INDEX_INSTALL_HOOK
        .get_or_init(|| std::sync::Mutex::new(Vec::new()))
        .lock()
        .expect("resolution index-install hook")
        .push((root, Box::new(hook)));
}

#[cfg(test)]
fn set_resolution_index_effect_hook(root: PathBuf, hook: impl FnOnce() + Send + 'static) {
    RESOLUTION_INDEX_EFFECT_HOOK
        .get_or_init(|| std::sync::Mutex::new(Vec::new()))
        .lock()
        .expect("resolution index-effect hook")
        .push((root, Box::new(hook)));
}

#[cfg(test)]
fn set_resolution_index_retire_hook(root: PathBuf, hook: impl FnOnce() + Send + 'static) {
    RESOLUTION_INDEX_RETIRE_HOOK
        .get_or_init(|| std::sync::Mutex::new(Vec::new()))
        .lock()
        .expect("resolution index-retire hook")
        .push((root, Box::new(hook)));
}

#[cfg(test)]
fn run_resolution_index_hook(hooks: &std::sync::OnceLock<ResolutionIndexLockHook>, root: &Path) {
    let hook = {
        let mut hooks = hooks
            .get_or_init(|| std::sync::Mutex::new(Vec::new()))
            .lock()
            .expect("resolution index hook");
        hooks
            .iter()
            .position(|(expected_root, _)| expected_root == root)
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
use std::{
    fmt,
    io::{Read, Seek, SeekFrom, Write},
    path::Path,
};

#[cfg(target_os = "linux")]
use sha1::{Digest, Sha1};
#[cfg(unix)]
use std::{
    ffi::CString,
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
            validate_context_worktree(
                &repository,
                &context,
                true,
                RepositoryOperation::RepositorySnapshot,
            )
            .map_err(|_| SynchronizationError::TargetNotMaterialized)?;
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
    // A new synchronization must never adopt a foreign merge, rebase, or
    // cherry-pick merely because libgit2's state cache or index looks clean.
    let foreign_state = [
        "MERGE_HEAD",
        "REBASE_HEAD",
        "CHERRY_PICK_HEAD",
        "rebase-apply",
        "rebase-merge",
    ]
    .iter()
    .any(|name| linked.path().join(name).exists() || linked.commondir().join(name).exists());
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
    Ok(linked)
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

/// A Git-compatible `index.lock` held across the final validation and every
/// owned effect. The lock descriptor and its identity are retained so a
/// pathname substitution cannot be adopted at persistence.
#[cfg(unix)]
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
            libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
        )
    };
    if fd < 0 {
        return Err(SynchronizationError::ExternalChange);
    }
    let mut file = unsafe { std::fs::File::from_raw_fd(fd) };
    let metadata = file
        .metadata()
        .map_err(|_| SynchronizationError::ExternalChange)?;
    if !metadata.is_file() {
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

#[cfg(unix)]
fn index_image_is_exact(observed: &IndexFileImage, expected: &IndexFileImage) -> bool {
    observed.device == expected.device
        && observed.inode == expected.inode
        && observed.bytes == expected.bytes
}

#[cfg(target_os = "linux")]
fn index_entries_match(left: &git2::Index, right: &git2::Index) -> bool {
    left.len() == right.len()
        && left.iter().zip(right.iter()).all(|(left, right)| {
            left.id == right.id && left.mode == right.mode && left.path == right.path
        })
}

#[cfg(target_os = "linux")]
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
    if Sha1::digest(&raw[..checksum_start]).as_slice() != &raw[checksum_start..] {
        return Err(SynchronizationError::ExternalChange);
    }
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
        // signatures are optional. This serializer may rebuild only these
        // documented advisory caches; every other extension is semantic or
        // unknown and must remain fail-closed rather than be discarded.
        let permitted = matches!(signature, b"TREE" | b"UNTR" | b"FSMN" | b"EOIE" | b"IEOT");
        if !permitted {
            return Err(SynchronizationError::ExternalChange);
        }
        offset = end;
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn serialized_index_bytes(index: &git2::Index) -> Result<Vec<u8>, SynchronizationError> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"DIRC");
    bytes.extend_from_slice(&2_u32.to_be_bytes());
    bytes.extend_from_slice(&(index.len() as u32).to_be_bytes());
    for entry in index.iter() {
        let entry_start = bytes.len();
        for value in [
            entry.ctime.seconds() as u32,
            entry.ctime.nanoseconds(),
            entry.mtime.seconds() as u32,
            entry.mtime.nanoseconds(),
            entry.dev,
            entry.ino,
            entry.mode,
            entry.uid,
            entry.gid,
            entry.file_size,
        ] {
            bytes.extend_from_slice(&value.to_be_bytes());
        }
        bytes.extend_from_slice(entry.id.as_bytes());
        let flags = (entry.flags & 0xf000) | (entry.path.len().min(0x0fff) as u16);
        bytes.extend_from_slice(&flags.to_be_bytes());
        if flags & 0x4000 != 0 {
            bytes.extend_from_slice(&entry.flags_extended.to_be_bytes());
        }
        if entry.path.contains(&0) {
            return Err(SynchronizationError::ExternalChange);
        }
        bytes.extend_from_slice(&entry.path);
        bytes.push(0);
        while (bytes.len() - entry_start) % 8 != 0 {
            bytes.push(0);
        }
    }
    let checksum = Sha1::digest(&bytes);
    bytes.extend_from_slice(&checksum);
    Ok(bytes)
}

#[cfg(target_os = "linux")]
fn serialized_index_descriptor(bytes: &[u8]) -> Result<std::fs::File, SynchronizationError> {
    let name = CString::new("manyhands-resolution-index").expect("fixed memfd name");
    let fd = unsafe { libc::syscall(libc::SYS_memfd_create, name.as_ptr(), libc::MFD_CLOEXEC) };
    if fd < 0 {
        return Err(SynchronizationError::ExternalChange);
    }
    let mut file = unsafe { std::fs::File::from_raw_fd(fd as std::os::fd::RawFd) };
    file.write_all(bytes)
        .and_then(|()| file.sync_all())
        .and_then(|()| file.seek(SeekFrom::Start(0)).map(|_| ()))
        .map_err(|_| SynchronizationError::ExternalChange)?;
    Ok(file)
}

#[cfg(target_os = "linux")]
fn rename_index_entry(
    parent: &std::fs::File,
    source: &CString,
    destination: &CString,
    flags: libc::c_uint,
) -> std::io::Result<()> {
    let result = unsafe {
        libc::syscall(
            libc::SYS_renameat2,
            parent.as_raw_fd(),
            source.as_ptr(),
            parent.as_raw_fd(),
            destination.as_ptr(),
            flags,
        )
    };
    if result == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

#[cfg(target_os = "linux")]
fn unique_index_retired_name() -> CString {
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_RETIRED: AtomicU64 = AtomicU64::new(0);
    CString::new(format!(
        ".manyhands-index-retired-{}-{}",
        std::process::id(),
        NEXT_RETIRED.fetch_add(1, Ordering::Relaxed)
    ))
    .expect("fixed retired index name")
}

#[cfg(target_os = "linux")]
fn unique_index_retire_placeholder_name() -> CString {
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_PLACEHOLDER: AtomicU64 = AtomicU64::new(0);
    CString::new(format!(
        ".manyhands-index-retire-placeholder-{}-{}",
        std::process::id(),
        NEXT_PLACEHOLDER.fetch_add(1, Ordering::Relaxed)
    ))
    .expect("fixed retire placeholder name")
}

#[cfg(target_os = "linux")]
fn unique_index_acquire_name() -> CString {
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_ACQUIRE: AtomicU64 = AtomicU64::new(0);
    CString::new(format!(
        ".manyhands-index-acquire-{}-{}",
        std::process::id(),
        NEXT_ACQUIRE.fetch_add(1, Ordering::Relaxed)
    ))
    .expect("fixed acquire index name")
}

#[cfg(target_os = "linux")]
fn index_retire_placeholder(
    parent: &std::fs::File,
) -> Result<(CString, IndexFileImage), SynchronizationError> {
    let name = unique_index_retire_placeholder_name();
    let fd = unsafe {
        libc::openat(
            parent.as_raw_fd(),
            name.as_ptr(),
            libc::O_RDWR | libc::O_CREAT | libc::O_EXCL | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            0o600,
        )
    };
    if fd < 0 {
        return Err(SynchronizationError::ExternalChange);
    }
    let mut file = unsafe { std::fs::File::from_raw_fd(fd) };
    let bytes = b"manyhands-index-retire-placeholder-v1\n".to_vec();
    file.write_all(&bytes)
        .and_then(|()| file.sync_all())
        .map_err(|_| SynchronizationError::ExternalChange)?;
    let metadata = file
        .metadata()
        .map_err(|_| SynchronizationError::ExternalChange)?;
    Ok((
        name,
        IndexFileImage {
            bytes,
            device: metadata.dev(),
            inode: metadata.ino(),
        },
    ))
}

#[cfg(target_os = "linux")]
fn guarded_retire_index_sentinel(
    parent: &std::fs::File,
    expected: &IndexFileImage,
) -> Result<(), SynchronizationError> {
    let lock_name = CString::new("index.lock").expect("fixed index name");
    let (_, current) = index_file_image_at(parent, &lock_name)?;
    if !index_image_is_exact(&current, expected) {
        return Err(SynchronizationError::ExternalChange);
    }
    let (placeholder_name, placeholder) = index_retire_placeholder(parent)?;
    // Recheck immediately before exchange. An external replacement before
    // this boundary remains at index.lock and is never retired.
    let (_, current) = index_file_image_at(parent, &lock_name)?;
    if !index_image_is_exact(&current, expected) {
        return Err(SynchronizationError::ExternalChange);
    }
    rename_index_entry(parent, &lock_name, &placeholder_name, 2)
        .map_err(|_| SynchronizationError::ExternalChange)?;
    let (_, lock_after_exchange) = index_file_image_at(parent, &lock_name)?;
    let (_, archived) = index_file_image_at(parent, &placeholder_name)?;
    if !index_image_is_exact(&archived, expected)
        || !index_image_is_exact(&lock_after_exchange, &placeholder)
    {
        // If the only observed postimage is our placeholder at index.lock,
        // exchange back to restore a substituted external lock atomically.
        if index_image_is_exact(&lock_after_exchange, &placeholder) {
            let _ = rename_index_entry(parent, &lock_name, &placeholder_name, 2);
        }
        let _ = parent.sync_all();
        return Err(SynchronizationError::ExternalChange);
    }
    // Linux has no conditional unlink. Move only the verified private
    // placeholder to a retained name; a later substitution is preserved under
    // that name and reported rather than path-unlinked.
    let released = unique_index_retired_name();
    rename_index_entry(parent, &lock_name, &released, 1)
        .map_err(|_| SynchronizationError::ExternalChange)?;
    let (_, released_image) = index_file_image_at(parent, &released)?;
    if !index_image_is_exact(&released_image, &placeholder) {
        let _ = parent.sync_all();
        return Err(SynchronizationError::ExternalChange);
    }
    parent
        .sync_all()
        .map_err(|_| SynchronizationError::ExternalChange)
}

#[cfg(target_os = "linux")]
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

#[cfg(target_os = "linux")]
fn index_from_descriptor(file: &std::fs::File) -> Result<git2::Index, SynchronizationError> {
    git2::Index::open(Path::new(&format!("/proc/self/fd/{}", file.as_raw_fd())))
        .map_err(|_| SynchronizationError::ExternalChange)
}

#[cfg(target_os = "linux")]
fn conflict_index_digest(
    index: &git2::Index,
    incoming: git2::Oid,
) -> Result<[u8; 32], SynchronizationError> {
    if !index.has_conflicts() {
        return Err(SynchronizationError::ExternalChange);
    }
    let mut digest = blake3::Hasher::new();
    for conflict in index
        .conflicts()
        .map_err(|_| SynchronizationError::ExternalChange)?
    {
        let conflict = conflict.map_err(|_| SynchronizationError::ExternalChange)?;
        for entry in [conflict.ancestor, conflict.our, conflict.their]
            .into_iter()
            .flatten()
        {
            digest.update(&entry.mode.to_le_bytes());
            digest.update(entry.id.as_bytes());
            digest.update(&entry.path);
        }
    }
    digest.update(incoming.as_bytes());
    Ok(*digest.finalize().as_bytes())
}

#[cfg(target_os = "linux")]
struct RecoveredResolutionIndexLock {
    parent: std::fs::File,
    final_index: IndexFileImage,
    sentinel: IndexFileImage,
}

#[cfg(target_os = "linux")]
impl RecoveredResolutionIndexLock {
    fn recognize(
        repository: &git2::Repository,
        candidate_tree: git2::Oid,
        local: git2::Oid,
        incoming: git2::Oid,
        conflict_digest: [u8; 32],
    ) -> Result<Option<Self>, SynchronizationError> {
        let root = CString::new(repository.path().as_os_str().as_bytes())
            .map_err(|_| SynchronizationError::ExternalChange)?;
        let fd = unsafe {
            libc::open(
                root.as_ptr(),
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            )
        };
        if fd < 0 {
            return Err(SynchronizationError::ExternalChange);
        }
        let parent = unsafe { std::fs::File::from_raw_fd(fd) };
        let lock_name = CString::new("index.lock").expect("fixed index name");
        if !index_leaf_present(&parent, &lock_name)? {
            return Ok(None);
        }
        let index_name = CString::new("index").expect("fixed index name");
        let (index_file, final_index) = index_file_image_at(&parent, &index_name)?;
        let (lock_file, sentinel) = index_file_image_at(&parent, &lock_name)?;
        let mut final_parsed = index_from_descriptor(&index_file)?;
        let sentinel_parsed = index_from_descriptor(&lock_file)?;
        if final_parsed.has_conflicts()
            || final_parsed
                .write_tree_to(repository)
                .map_err(|_| SynchronizationError::ExternalChange)?
                != candidate_tree
            || !sentinel_parsed.has_conflicts()
            || !recorded_merge_index_matches_index(repository, &sentinel_parsed, local, incoming)?
        {
            return Err(SynchronizationError::ExternalChange);
        }
        if conflict_index_digest(&sentinel_parsed, incoming)? != conflict_digest {
            return Err(SynchronizationError::ExternalChange);
        }
        Ok(Some(Self {
            parent,
            final_index,
            sentinel,
        }))
    }

    fn images_match(&self) -> Result<(), SynchronizationError> {
        let index_name = CString::new("index").expect("fixed index name");
        let (_, index) = index_file_image_at(&self.parent, &index_name)?;
        let lock_name = CString::new("index.lock").expect("fixed index name");
        let (_, sentinel) = index_file_image_at(&self.parent, &lock_name)?;
        if index_image_is_exact(&index, &self.final_index)
            && index_image_is_exact(&sentinel, &self.sentinel)
        {
            Ok(())
        } else {
            Err(SynchronizationError::ExternalChange)
        }
    }

    fn retire(self) -> Result<(), SynchronizationError> {
        self.images_match()?;
        guarded_retire_index_sentinel(&self.parent, &self.sentinel)
    }
}

#[cfg(target_os = "linux")]
struct EarlyResolutionIndexLock {
    parent: std::fs::File,
    image: IndexFileImage,
}

#[cfg(target_os = "linux")]
impl EarlyResolutionIndexLock {
    fn recognize(
        repository: &git2::Repository,
        local: git2::Oid,
        incoming: git2::Oid,
        conflict_digest: [u8; 32],
    ) -> Result<Option<Self>, SynchronizationError> {
        let root = CString::new(repository.path().as_os_str().as_bytes())
            .map_err(|_| SynchronizationError::ExternalChange)?;
        let fd = unsafe {
            libc::open(
                root.as_ptr(),
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            )
        };
        if fd < 0 {
            return Err(SynchronizationError::ExternalChange);
        }
        let parent = unsafe { std::fs::File::from_raw_fd(fd) };
        let lock_name = CString::new("index.lock").expect("fixed index name");
        if !index_leaf_present(&parent, &lock_name)? {
            return Ok(None);
        }
        let index_name = CString::new("index").expect("fixed index name");
        let (index_file, index) = index_file_image_at(&parent, &index_name)?;
        let (lock_file, image) = index_file_image_at(&parent, &lock_name)?;
        // An early owned lock is the fully initialized preflight image copied
        // atomically at acquire. The live index must still be the exact same
        // conflict image; anything else is an external lock.
        if index.bytes != image.bytes {
            return Err(SynchronizationError::ExternalChange);
        }
        let index = index_from_descriptor(&index_file)?;
        let lock = index_from_descriptor(&lock_file)?;
        if !index.has_conflicts()
            || !lock.has_conflicts()
            || conflict_index_digest(&lock, incoming)? != conflict_digest
            || !recorded_merge_index_matches_index(repository, &lock, local, incoming)?
        {
            return Err(SynchronizationError::ExternalChange);
        }
        Ok(Some(Self { parent, image }))
    }

    fn retire(self) -> Result<(), SynchronizationError> {
        guarded_retire_index_sentinel(&self.parent, &self.image)
    }
}

struct ResolutionIndexLock {
    active: bool,
    #[cfg(test)]
    workdir: PathBuf,
    #[cfg(unix)]
    parent: std::fs::File,
    #[cfg(unix)]
    lock: std::fs::File,
    #[cfg(unix)]
    lock_device: u64,
    #[cfg(unix)]
    lock_inode: u64,
    #[cfg(unix)]
    original_index: IndexFileImage,
    #[cfg(unix)]
    persisted_index: Option<IndexFileImage>,
}

impl ResolutionIndexLock {
    #[cfg(target_os = "linux")]
    fn acquire(repository: &git2::Repository) -> Result<Self, SynchronizationError> {
        let root = CString::new(repository.path().as_os_str().as_bytes())
            .map_err(|_| SynchronizationError::ExternalChange)?;
        let parent_fd = unsafe {
            libc::open(
                root.as_ptr(),
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            )
        };
        if parent_fd < 0 {
            return Err(SynchronizationError::ExternalChange);
        }
        let parent = unsafe { std::fs::File::from_raw_fd(parent_fd) };
        let index_name = CString::new("index").expect("fixed index name");
        let (_, original_index) = index_file_image_at(&parent, &index_name)?;
        approved_index_extensions(&original_index.bytes)?;
        // Fully initialize a private leaf before publishing index.lock. This
        // removes the crash window where a named but empty lock cannot be
        // distinguished from an external Git writer's lock on restart.
        let acquire_name = unique_index_acquire_name();
        let lock_fd = unsafe {
            libc::openat(
                parent.as_raw_fd(),
                acquire_name.as_ptr(),
                libc::O_RDWR | libc::O_CREAT | libc::O_EXCL | libc::O_NOFOLLOW | libc::O_CLOEXEC,
                0o600,
            )
        };
        if lock_fd < 0 {
            return Err(SynchronizationError::ExternalChange);
        }
        let mut lock = unsafe { std::fs::File::from_raw_fd(lock_fd) };
        lock.write_all(&original_index.bytes)
            .and_then(|()| lock.sync_all())
            .map_err(|_| SynchronizationError::ExternalChange)?;
        let metadata = lock
            .metadata()
            .map_err(|_| SynchronizationError::ExternalChange)?;
        if !metadata.is_file() {
            return Err(SynchronizationError::ExternalChange);
        }
        let lock_name = CString::new("index.lock").expect("fixed lock name");
        rename_index_entry(&parent, &acquire_name, &lock_name, 1)
            .map_err(|_| SynchronizationError::ExternalChange)?;
        let (_, published) = index_file_image_at(&parent, &lock_name)?;
        if published.device != metadata.dev()
            || published.inode != metadata.ino()
            || published.bytes != original_index.bytes
        {
            return Err(SynchronizationError::ExternalChange);
        }
        #[cfg(test)]
        run_resolution_index_lock_hook(
            repository
                .workdir()
                .ok_or(SynchronizationError::ExternalChange)?,
        );
        Ok(Self {
            active: true,
            #[cfg(test)]
            workdir: repository
                .workdir()
                .ok_or(SynchronizationError::ExternalChange)?
                .to_owned(),
            parent,
            lock,
            lock_device: metadata.dev(),
            lock_inode: metadata.ino(),
            original_index,
            persisted_index: None,
        })
    }

    #[cfg(not(target_os = "linux"))]
    fn acquire(_repository: &git2::Repository) -> Result<Self, SynchronizationError> {
        Err(SynchronizationError::ExternalChange)
    }

    #[cfg(unix)]
    fn lock_image(&self) -> Result<IndexFileImage, SynchronizationError> {
        let descriptor = self
            .lock
            .metadata()
            .map_err(|_| SynchronizationError::ExternalChange)?;
        if !descriptor.is_file()
            || descriptor.dev() != self.lock_device
            || descriptor.ino() != self.lock_inode
        {
            return Err(SynchronizationError::ExternalChange);
        }
        let lock_name = CString::new("index.lock").expect("fixed lock name");
        let (_, image) = index_file_image_at(&self.parent, &lock_name)?;
        if image.device != self.lock_device || image.inode != self.lock_inode {
            return Err(SynchronizationError::ExternalChange);
        }
        Ok(image)
    }

    #[cfg(unix)]
    fn lock_contains(&self, bytes: &[u8]) -> Result<(), SynchronizationError> {
        (self.lock_image()?.bytes == bytes)
            .then_some(())
            .ok_or(SynchronizationError::ExternalChange)
    }

    #[cfg(unix)]
    fn index_contains_original(&self) -> Result<(), SynchronizationError> {
        let index_name = CString::new("index").expect("fixed index name");
        let (_, image) = index_file_image_at(&self.parent, &index_name)?;
        index_image_is_exact(&image, &self.original_index)
            .then_some(())
            .ok_or(SynchronizationError::ExternalChange)
    }

    #[cfg(target_os = "linux")]
    fn persisted_images_match(&self) -> Result<(), SynchronizationError> {
        let expected_index = self
            .persisted_index
            .as_ref()
            .ok_or(SynchronizationError::ExternalChange)?;
        let index_name = CString::new("index").expect("fixed index name");
        let (_, index) = index_file_image_at(&self.parent, &index_name)?;
        let lock_name = CString::new("index.lock").expect("fixed lock name");
        let (_, sentinel) = index_file_image_at(&self.parent, &lock_name)?;
        if index_image_is_exact(&index, expected_index)
            && index_image_is_exact(&sentinel, &self.original_index)
        {
            Ok(())
        } else {
            Err(SynchronizationError::ExternalChange)
        }
    }

    #[cfg(target_os = "linux")]
    fn retire(&mut self) -> Result<(), SynchronizationError> {
        // This is the final compare-and-retire boundary. The sentinel remains
        // visible to cooperating Git writers until every owned effect has
        // completed and both exchanged images are exact.
        self.persisted_images_match()?;
        #[cfg(test)]
        run_resolution_index_retire_hook(&self.workdir);
        // The hook is intentionally between precheck and guarded exchange.
        // Revalidate before any retirement so an external index.lock remains.
        self.persisted_images_match()?;
        guarded_retire_index_sentinel(&self.parent, &self.original_index)?;
        self.active = false;
        Ok(())
    }

    #[cfg(target_os = "linux")]
    fn persist(
        &mut self,
        source: &git2::Index,
        _repository: &git2::Repository,
    ) -> Result<(), SynchronizationError> {
        self.index_contains_original()?;
        // Serialize the stage-zero source index into an anonymous memfd. The
        // descriptor is the only serialization authority: there is no scratch
        // leaf for a persistent rename replacement to race with libgit2.
        let scratch_bytes = serialized_index_bytes(source)?;
        let scratch = serialized_index_descriptor(&scratch_bytes)?;
        #[cfg(test)]
        run_resolution_index_scratch_hook(
            _repository
                .workdir()
                .ok_or(SynchronizationError::ExternalChange)?,
        );
        let parsed =
            git2::Index::open(Path::new(&format!("/proc/self/fd/{}", scratch.as_raw_fd())))
                .map_err(|_| SynchronizationError::ExternalChange)?;
        if !index_entries_match(&parsed, source) {
            return Err(SynchronizationError::ExternalChange);
        }
        // Only bytes read from the no-follow scratch descriptor are copied.
        self.lock
            .set_len(0)
            .and_then(|()| self.lock.seek(SeekFrom::Start(0)).map(|_| ()))
            .and_then(|()| self.lock.write_all(&scratch_bytes))
            .and_then(|()| self.lock.sync_all())
            .map_err(|_| SynchronizationError::ExternalChange)?;
        self.lock_contains(&scratch_bytes)?;
        self.index_contains_original()?;
        #[cfg(test)]
        run_resolution_index_persist_hook(
            _repository
                .workdir()
                .ok_or(SynchronizationError::ExternalChange)?,
        );
        self.lock_contains(&scratch_bytes)?;
        self.index_contains_original()?;
        let lock_name = CString::new("index.lock").expect("fixed lock name");
        let index_name = CString::new("index").expect("fixed index name");
        rename_index_entry(&self.parent, &lock_name, &index_name, 2)
            .map_err(|_| SynchronizationError::ExternalChange)?;
        let (_, installed) = index_file_image_at(&self.parent, &index_name)?;
        let (_, displaced) = index_file_image_at(&self.parent, &lock_name)?;
        if installed.bytes != scratch_bytes
            || !index_image_is_exact(&displaced, &self.original_index)
        {
            self.parent
                .sync_all()
                .map_err(|_| SynchronizationError::ExternalChange)?;
            return Err(SynchronizationError::ExternalChange);
        }
        // After exchange the lock pathname is the original index sentinel.
        // Keep it there through checkout, ref CAS, and checkpoint persistence.
        self.persisted_index = Some(installed);
        #[cfg(test)]
        run_resolution_index_install_hook(
            _repository
                .workdir()
                .ok_or(SynchronizationError::ExternalChange)?,
        );
        self.persisted_images_match()
    }

    #[cfg(not(target_os = "linux"))]
    fn persist(
        &mut self,
        _source: &git2::Index,
        _repository: &git2::Repository,
    ) -> Result<(), SynchronizationError> {
        Err(SynchronizationError::ExternalChange)
    }

    #[cfg(not(target_os = "linux"))]
    fn persisted_images_match(&self) -> Result<(), SynchronizationError> {
        Err(SynchronizationError::ExternalChange)
    }

    #[cfg(not(target_os = "linux"))]
    fn retire(&mut self) -> Result<(), SynchronizationError> {
        Err(SynchronizationError::ExternalChange)
    }
}

impl Drop for ResolutionIndexLock {
    fn drop(&mut self) {
        #[cfg(target_os = "linux")]
        if self.active && self.persisted_index.is_none() {
            // Before exchange, move only a still-recognized owned lock out of
            // Git's lock name. After exchange, preserve the old-index sentinel
            // until the explicit final retire path completes.
            if let Ok(image) = self.lock_image() {
                let _ = guarded_retire_index_sentinel(&self.parent, &image);
            }
        }
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
    match owned_file_bytes(
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

fn owned_result_digest(root: &Path, relative: &Path) -> Result<[u8; 32], SynchronizationError> {
    let bytes = owned_file_bytes(
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
        match owned_file_bytes(
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
    let mut options = resolution_status_options();
    for entry in repository
        .statuses(Some(&mut options))
        .map_err(|_| SynchronizationError::RecoveryRequired)?
        .iter()
    {
        if entry.status().is_ignored() {
            continue;
        }
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
    let local = repository
        .find_commit(local)
        .map_err(|_| SynchronizationError::RecoveryRequired)?;
    let incoming = repository
        .find_commit(incoming)
        .map_err(|_| SynchronizationError::RecoveryRequired)?;
    let expected = repository
        .merge_commits(&local, &incoming, None)
        .map_err(|_| SynchronizationError::RecoveryRequired)?;
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
    let mut expected_entries = entries(&expected);
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

/// Only the exact merge that this integration step recorded may be retired.
/// Foreign rebase/cherry-pick state is never cleanup authority.
fn exact_resolution_merge_metadata(
    repository: &mut git2::Repository,
    incoming: git2::Oid,
) -> Result<bool, SynchronizationError> {
    if ["REBASE_HEAD", "CHERRY_PICK_HEAD"].iter().any(|name| {
        repository.path().join(name).exists() || repository.commondir().join(name).exists()
    }) {
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

fn resolution_merge_metadata_absent(repository: &git2::Repository) -> bool {
    ["MERGE_HEAD", "REBASE_HEAD", "CHERRY_PICK_HEAD"]
        .iter()
        .all(|name| {
            !repository.path().join(name).exists() && !repository.commondir().join(name).exists()
        })
}

fn conflict_digest(repository: &mut git2::Repository) -> Result<[u8; 32], SynchronizationError> {
    let index = repository
        .index()
        .map_err(|_| SynchronizationError::RecoveryRequired)?;
    if !index.has_conflicts() {
        return Err(SynchronizationError::RecoveryRequired);
    }
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
    repository
        .mergehead_foreach(|oid| {
            digest.update(oid.as_bytes());
            true
        })
        .map_err(|_| SynchronizationError::RecoveryRequired)?;
    Ok(*digest.finalize().as_bytes())
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
    ordinal: u8,
    oid: git2::Oid,
    tree: git2::Oid,
}

/// Observe a candidate's local ref transition on restart, but keep the outer
/// synchronization envelope reconciling until Fetch is observed again. The
/// re-fetch is mandatory before any push/publication continuation.
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
    let candidate = step
        .candidate_oid
        .ok_or(SynchronizationError::RecoveryRequired)?;
    let repository = local_target(root, primary_branch, target)?;
    let _lease = repository_lease(&repository, root, RepositoryOperation::RepositorySnapshot)?;
    let head = local_oid(&repository)?;
    let commit = repository
        .find_commit(candidate)
        .map_err(|_| SynchronizationError::RecoveryRequired)?;
    let parents = [commit.parent_id(0).ok(), commit.parent_id(1).ok()];
    if commit.parent_count() != 2
        || parents != [Some(step.intent.local_oid), Some(step.intent.incoming_oid)]
    {
        return Err(SynchronizationError::RecoveryRequired);
    }
    let tree = commit.tree_id();
    if head == step.intent.local_oid {
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
        fast_forward(&repository, &branch, head, candidate)?;
    } else if head != candidate {
        return Err(SynchronizationError::RecoveryRequired);
    }
    let observed = local_oid(&repository)?;
    if observed != candidate {
        return Err(SynchronizationError::RecoveryRequired);
    }
    service.observe_synchronization_integration_effect(
        root,
        owner,
        step.intent.ordinal,
        candidate,
        tree,
    )?;
    evidence.local_oid = Some(candidate);
    Ok(Some(ReconciledCandidate {
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
    decision(service.reconcile_synchronization_candidate_applied(
        root,
        owner,
        candidate.ordinal,
        candidate.oid,
        candidate.tree,
        evidence,
    )?)
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
    let target_repository = local_target(input.root, input.primary_branch, input.target)?;
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
    let stages = merge::integration_stages(target, context.is_some());
    for (ordinal, stage) in stages.into_iter().enumerate() {
        let incoming = match stage {
            merge::IntegrationStage::Context => {
                context.ok_or(SynchronizationError::RecoveryRequired)?
            }
            merge::IntegrationStage::Primary => primary,
        };
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
            ordinal: ordinal as u8,
            stage,
            local_oid: local,
            incoming_oid: incoming,
            baseline_tree_oid: local_commit.tree_id(),
            baseline_index_digest: index_digest(local_commit.tree_id()),
        };
        drop(local_commit);
        // Persist the exact ordered parents before even preparing a candidate;
        // a restart must never synthesize another candidate for this stage.
        service.prepare_synchronization_integration(root, owner, &intent)?;
        let prepared = if classification == merge::IntegrationDisposition::MergeRequired {
            Some(prepare_clean_merge(&path, local, incoming)?)
        } else {
            None
        };
        match classification {
            merge::IntegrationDisposition::Equal
            | merge::IntegrationDisposition::IncomingAlreadyIntegrated => {
                service.begin_synchronization_integration_effect(
                    root,
                    owner,
                    ordinal as u8,
                    None,
                )?;
                let tree = target_repository
                    .find_commit(local)
                    .map_err(|_| SynchronizationError::RecoveryRequired)?
                    .tree_id();
                service.observe_synchronization_integration_effect(
                    root,
                    owner,
                    ordinal as u8,
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
                recheck_divergence_inputs(service, &input, local)?;
                if target_repository.find_commit(incoming).is_err() {
                    return Err(SynchronizationError::ExternalChange);
                }
                service.begin_synchronization_integration_effect(
                    root,
                    owner,
                    ordinal as u8,
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
                service.observe_synchronization_integration_effect(
                    root,
                    owner,
                    ordinal as u8,
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
                    service.begin_synchronization_integration_effect(
                        root,
                        owner,
                        ordinal as u8,
                        Some(candidate),
                    )?;
                    let branch = repository
                        .find_reference("HEAD")
                        .ok()
                        .and_then(|head| head.symbolic_target().map(str::to_owned))
                        .ok_or(SynchronizationError::ExternalChange)?;
                    fast_forward(&repository, &branch, local, candidate)?;
                    service.observe_synchronization_integration_effect(
                        root,
                        owner,
                        ordinal as u8,
                        candidate,
                        tree.id(),
                    )?;
                    local = candidate;
                } else {
                    service.begin_synchronization_integration_effect(
                        root,
                        owner,
                        ordinal as u8,
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
                    let fingerprint = conflict_digest(&mut repository)?;
                    service.release_synchronization_conflict(
                        root,
                        owner,
                        ordinal as u8,
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
        let (path, branch) = match target {
            SynchronizationTarget::Primary => (root.to_owned(), config.primary_branch),
            SynchronizationTarget::Context { kind, item_id } => (
                root.join(".manyhands/worktrees").join(item_id.to_string()),
                format!("manyhands/{}/{}", authoring_kind_segment(kind), item_id),
            ),
        };
        let repository =
            git2::Repository::open(path).map_err(|_| SynchronizationError::RecoveryRequired)?;
        if repository
            .find_reference("HEAD")
            .ok()
            .and_then(|head| head.symbolic_target().map(str::to_owned))
            != Some(format!("refs/heads/{branch}"))
        {
            return Err(SynchronizationError::ExternalChange);
        }
        Ok(repository)
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
            }
            _ => false,
        }
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
        for (token, bytes) in replacements.values() {
            sources.insert(token.path.clone(), bytes.bytes().to_vec());
        }
        let context =
            canonical::validate_context(sources.into_iter().filter_map(|(path, bytes)| {
                let path = std::str::from_utf8(&path).ok()?.to_owned();
                let text = String::from_utf8(bytes).ok()?;
                Some((std::path::PathBuf::from(path), text))
            }));
        Ok(context.problems.is_empty())
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
            for ordinal in 0..=1 {
                if let Some(step) = state::integration_step(tx, record.id, ordinal)?
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
        let fingerprint = conflict_digest(&mut repository)?;
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
            || conflict_digest(&mut repository)? != token.observation.fingerprint
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
        let mut repository = Self::inspect_conflict_target(root, &target)?;
        let _lease = repository_lease(&repository, root, RepositoryOperation::RepositorySnapshot)?;
        let metadata_exact =
            exact_resolution_merge_metadata(&mut repository, step.intent.incoming_oid)?;
        if !metadata_exact && !(phase == "applied" && resolution_merge_metadata_absent(&repository))
        {
            return Err(SynchronizationError::ExternalChange);
        }
        let current_head = local_oid(&repository)?;
        if current_head != candidate {
            if phase == "candidate_prepared" && current_head == step.intent.local_oid {
                #[cfg(target_os = "linux")]
                if let Some(lock) = EarlyResolutionIndexLock::recognize(
                    &repository,
                    step.intent.local_oid,
                    step.intent.incoming_oid,
                    step.conflict_digest
                        .ok_or(SynchronizationError::RecoveryRequired)?,
                )? {
                    lock.retire()?;
                }
                return Ok(None);
            }
            return Err(SynchronizationError::ExternalChange);
        }
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
        #[cfg(target_os = "linux")]
        let recovered_index_lock = RecoveredResolutionIndexLock::recognize(
            &repository,
            tree_id,
            step.intent.local_oid,
            step.intent.incoming_oid,
            step.conflict_digest
                .ok_or(SynchronizationError::RecoveryRequired)?,
        )?;
        let owner = match self.reacquire_synchronization_conflict(
            root,
            request.synchronization_id,
            &operation_target,
            request.observation.ordinal,
            request.observation.fingerprint,
        )? {
            RemoteReservationOutcome::Reserved(owner) => owner,
            RemoteReservationOutcome::Busy => return Err(SynchronizationError::Busy),
            _ => return Err(SynchronizationError::RecoveryRequired),
        };
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
        let metadata_exact =
            exact_resolution_merge_metadata(&mut repository, step.intent.incoming_oid)?;
        if !metadata_exact && !(phase == "applied" && resolution_merge_metadata_absent(&repository))
        {
            return Err(SynchronizationError::ExternalChange);
        }
        #[cfg(target_os = "linux")]
        if let Some(lock) = recovered_index_lock.as_ref() {
            // Recheck both images after durable candidate reconciliation and
            // retire while the merge metadata still makes ownership provable.
            lock.images_match()?;
        }
        #[cfg(target_os = "linux")]
        if let Some(lock) = recovered_index_lock {
            lock.retire()?;
        }
        self.check_failure(
            FailurePoint::ResolutionAfterIndexLockRetirement,
            RepositoryOperation::RepositorySnapshot,
            root,
        )?;
        if metadata_exact {
            repository
                .cleanup_state()
                .map_err(|_| SynchronizationError::RecoveryRequired)?;
        }
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
            let Ok(text) = std::str::from_utf8(bytes.bytes()) else {
                return Ok(merge::ResolveSynchronizationOutcome::ValidationFailed);
            };
            let Ok(item) = canonical::parse_item(
                std::path::Path::new(std::str::from_utf8(&token.path).unwrap_or_default()),
                text,
            ) else {
                return Ok(merge::ResolveSynchronizationOutcome::ValidationFailed);
            };
            let side_ids = [token.base, token.local, token.incoming];
            let Some(side_oid) = side_ids.into_iter().flatten().next() else {
                return Ok(merge::ResolveSynchronizationOutcome::ValidationFailed);
            };
            let blob = preflight_repository
                .find_blob(side_oid)
                .map_err(|_| SynchronizationError::RecoveryRequired)?;
            let Ok(side_text) = std::str::from_utf8(blob.content()) else {
                return Ok(merge::ResolveSynchronizationOutcome::ValidationFailed);
            };
            let Ok(expected) = canonical::parse_item(
                std::path::Path::new(std::str::from_utf8(&token.path).unwrap_or_default()),
                side_text,
            ) else {
                return Ok(merge::ResolveSynchronizationOutcome::ValidationFailed);
            };
            if Self::canonical_item_id(&item) != Self::canonical_item_id(&expected)
                || !Self::preserves_item_invariants(&expected, &item)
            {
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
        let owner = match self.reacquire_synchronization_conflict(
            &root,
            request.synchronization_id,
            &target,
            request.observation.ordinal,
            request.observation.fingerprint,
        )? {
            RemoteReservationOutcome::Reserved(owner) => owner,
            RemoteReservationOutcome::Busy => return Err(SynchronizationError::Busy),
            _ => return Ok(merge::ResolveSynchronizationOutcome::RecoveryRequired),
        };
        let mut repository = Self::inspect_conflict_target(&root, &inspection.target)?;
        let _lease = repository_lease(&repository, &root, RepositoryOperation::RepositorySnapshot)?;
        if local_oid(&repository)? != request.observation.head
            || Self::conflict_configuration(&root)? != request.observation.configuration
            || conflict_digest(&mut repository)? != request.observation.fingerprint
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
        self.prepare_synchronization_resolution_attempt(
            &root,
            &owner,
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
        let mut index_lock = ResolutionIndexLock::acquire(&repository)?;
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

        let mut index = repository
            .index()
            .map_err(|_| SynchronizationError::RecoveryRequired)?;
        for (path, durable) in inspection.paths.iter().zip(&durable_paths) {
            let (token, result) = supplied
                .get(&path.token.ordinal)
                .expect("validated complete tokens");
            let relative = std::path::Path::new(
                std::str::from_utf8(&token.path)
                    .map_err(|_| SynchronizationError::RecoveryRequired)?,
            );
            let workdir = repository
                .workdir()
                .ok_or(SynchronizationError::RecoveryRequired)?;
            let observed = if durable.applied {
                owned_result_digest(workdir, relative)?
            } else {
                conflict_worktree_digest(workdir, relative)?
            };
            let expected = if durable.applied {
                durable.result_digest
            } else {
                durable.prewrite_digest
            };
            if durable.ordinal != token.ordinal
                || *blake3::hash(result.bytes()).as_bytes() != durable.result_digest
                || observed != expected
            {
                return Err(SynchronizationError::ExternalChange);
            }
            // A resumed effect never writes an already-applied path. For a
            // remaining path the descriptor baseline above must still be the
            // durable pre-write image before the guarded replacement.
            if !durable.applied {
                write_owned_document_if_prewrite_digest(
                    repository
                        .workdir()
                        .ok_or(SynchronizationError::RecoveryRequired)?,
                    relative,
                    result.bytes(),
                    durable.prewrite_digest,
                    RepositoryOperation::RepositorySnapshot,
                    &root,
                )?;
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
            if !durable.applied {
                // Make the completed write durable before a fault can expose
                // a retry. A resumed attempt can then prove it must not write
                // this path again.
                self.observe_synchronization_resolution_path_effect(
                    &root,
                    &owner,
                    request.attempt_id,
                    token.ordinal,
                )?;
                self.check_failure(
                    FailurePoint::ResolutionAfterPathWrite,
                    RepositoryOperation::RepositorySnapshot,
                    &root,
                )?;
            }
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
        let signature = git2::Signature::new(&identity.0, &identity.1, &git2::Time::new(0, 0))
            .map_err(|_| SynchronizationError::RecoveryRequired)?;
        let candidate = repository
            .commit(
                None,
                &signature,
                &signature,
                "Resolve synchronization conflict",
                &tree,
                &[&head, &incoming],
            )
            .map_err(|_| SynchronizationError::RecoveryRequired)?;
        self.prepare_synchronization_resolution_candidate(
            &root,
            &owner,
            request.attempt_id,
            candidate,
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
        index_lock.persist(&index, &repository)?;
        #[cfg(test)]
        run_resolution_index_effect_hook(
            repository
                .workdir()
                .ok_or(SynchronizationError::RecoveryRequired)?,
        );
        // Reject a post-exchange external index or sentinel substitution
        // before checkout/ref mutation; the active sentinel blocks ordinary
        // cooperating Git writers throughout the remaining effects.
        index_lock.persisted_images_match()?;
        let mut checkout = git2::build::CheckoutBuilder::new();
        // The exchanged index is already durable and its old image remains at
        // index.lock as the writer sentinel; checkout must not reopen/write it.
        checkout.safe().overwrite_ignored(false).update_index(false);
        repository
            .checkout_index(Some(&mut index), Some(&mut checkout))
            .map_err(|_| SynchronizationError::RecoveryRequired)?;
        repository
            .reference_matching(&name, candidate, true, head.id(), "manyhands resolution")
            .map_err(|_| SynchronizationError::ExternalChange)?;
        index_lock.persisted_images_match()?;
        drop(head_ref);
        self.check_failure(
            FailurePoint::ResolutionAfterRefTransition,
            RepositoryOperation::RepositorySnapshot,
            &root,
        )?;
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
        // Retire while the exact MERGE_HEAD still proves the sentinel belongs
        // to this resolution. Recovery accepts the durable-applied/no-metadata
        // state if a crash lands after this point.
        index_lock.retire()?;
        self.check_failure(
            FailurePoint::ResolutionAfterIndexLockRetirement,
            RepositoryOperation::RepositorySnapshot,
            &root,
        )?;
        repository
            .cleanup_state()
            .map_err(|_| SynchronizationError::RecoveryRequired)?;
        self.check_failure(
            FailurePoint::ResolutionAfterMetadataCleanup,
            RepositoryOperation::RepositorySnapshot,
            &root,
        )?;
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
        let reservation = self.reserve_remote_operation(&root, request.operation_id, &target)?;
        let mut prior = None;
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
        let mut evidence = prior
            .as_ref()
            .map(|r| r.sync_evidence().clone())
            .unwrap_or(Evidence {
                expected_oid: Some(initial),
                local_oid: Some(initial),
                ..Evidence::default()
            });
        let pending_candidate = if prior.is_some() {
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
        // Preserve Cycle 05's all-clean virtual plan. Only its deliberate
        // divergence result enters the ordered, durable merge path.
        let divergence = {
            let local =
                self.inspect_synchronization_local(&root, &config.primary_branch, &request.target)?;
            matches!(
                graph_plan(
                    &repository,
                    &request.target,
                    local,
                    primary,
                    context,
                    publication,
                ),
                Err(SynchronizationError::MergeRequired { .. })
            )
        };
        if divergence {
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
