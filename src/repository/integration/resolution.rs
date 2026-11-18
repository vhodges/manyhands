//! Resolution path evidence, independent of token encoding and journal schema.
use super::Error;
use crate::repository::{RepositoryOperation, owned_resolution_file_bytes};
use std::path::Path;

pub(in super::super) fn preflight_path_bytes(path: &Path) -> Vec<u8> {
    path.to_string_lossy().as_bytes().to_vec()
}

/// Fingerprint the complete index/status and each capability-bound path before
/// attempt acceptance, then again under the lease. No caller bytes are persisted.
pub(in super::super) fn resolution_preflight(
    repository: &git2::Repository,
    paths: &[(u32, &[u8])],
) -> Result<[u8; 32], Error> {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"manyhands-resolution-preflight-v1\0");
    let index = repository.index().map_err(|_| Error::RecoveryRequired)?;
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
        .map_err(|_| Error::RecoveryRequired)?;
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
    let workdir = repository.workdir().ok_or(Error::RecoveryRequired)?;
    let mut paths = paths.to_vec();
    paths.sort_by_key(|(ordinal, _)| *ordinal);
    if paths.windows(2).any(|pair| pair[0].0 == pair[1].0) {
        return Err(Error::RecoveryRequired);
    }
    hasher.update(&(paths.len() as u64).to_be_bytes());
    for (ordinal, raw_path) in paths {
        let path_text = std::str::from_utf8(raw_path).map_err(|_| Error::ExternalChange)?;
        let path = Path::new(path_text);
        if path.as_os_str().is_empty()
            || !path
                .components()
                .all(|part| matches!(part, std::path::Component::Normal(_)))
        {
            return Err(Error::ExternalChange);
        }
        preflight_bytes(&mut hasher, &ordinal.to_be_bytes());
        preflight_bytes(&mut hasher, raw_path);
        // The same descriptor verifies and reads the image without following links.
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
            Err(_) => return Err(Error::ExternalChange),
        }
    }
    Ok(*hasher.finalize().as_bytes())
}

/// Ignored files remain permitted; authoritative merge-index comparison fences
/// unrelated staging while this policy fences worktree/untracked changes.
pub(in super::super) fn resolution_status_options() -> git2::StatusOptions {
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

pub(in super::super) fn only_paths_are_dirty(
    repository: &git2::Repository,
    allowed: &std::collections::BTreeSet<Vec<u8>>,
) -> Result<bool, Error> {
    let mut options = resolution_status_options();
    for entry in repository
        .statuses(Some(&mut options))
        .map_err(|_| Error::RecoveryRequired)?
        .iter()
    {
        if entry.status().is_ignored() {
            continue;
        }
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

pub(in super::super) fn preflight_bytes(hasher: &mut blake3::Hasher, bytes: &[u8]) {
    hasher.update(&(bytes.len() as u64).to_be_bytes());
    hasher.update(bytes);
}

pub(in super::super) fn conflict_worktree_digest(
    root: &Path,
    relative: &Path,
) -> Result<[u8; 32], Error> {
    let bytes = owned_resolution_file_bytes(
        root,
        relative,
        RepositoryOperation::RepositorySnapshot,
        root,
    )
    .map_err(|_| Error::ExternalChange)?;
    Ok(prewrite_image_digest(bytes.as_deref()))
}

/// The immutable preimage wire format: domain, presence, length and bytes.
/// Kept separate from the raw content digest used for an intended output.
fn prewrite_image_digest(bytes: Option<&[u8]>) -> [u8; 32] {
    let mut digest = blake3::Hasher::new();
    digest.update(b"manyhands-resolution-prewrite-v1\0");
    match bytes {
        Some(bytes) => {
            digest.update(&[1]);
            preflight_bytes(&mut digest, bytes);
        }
        None => {
            digest.update(&[0]);
        }
    }
    *digest.finalize().as_bytes()
}

/// The adapter has matched path/ordinal to its immutable capability and durable
/// intent. This checks the actual no-follow image, never a display-path string.
pub(in super::super) fn path_needs_write(
    root: &Path,
    relative: &Path,
    result: &[u8],
    result_digest: [u8; 32],
    prewrite_digest: [u8; 32],
    applied: bool,
) -> Result<bool, Error> {
    if *blake3::hash(result).as_bytes() != result_digest {
        return Err(Error::ExternalChange);
    }
    let bytes = owned_resolution_file_bytes(
        root,
        relative,
        RepositoryOperation::RepositorySnapshot,
        root,
    )
    .map_err(|_| Error::ExternalChange)?;
    path_image_needs_write(bytes.as_deref(), result_digest, prewrite_digest, applied)
}

/// Classifies an already-read no-follow image against the executor's intent.
/// A prepared path can still be at its preimage; an observed path cannot.
pub(in super::super) fn path_image_needs_write(
    bytes: Option<&[u8]>,
    result_digest: [u8; 32],
    prewrite_digest: [u8; 32],
    applied: bool,
) -> Result<bool, Error> {
    if bytes.map(|bytes| *blake3::hash(bytes).as_bytes()) == Some(result_digest) {
        return Ok(false);
    }
    if !applied && prewrite_image_digest(bytes) == prewrite_digest {
        return Ok(true);
    }
    Err(Error::ExternalChange)
}

pub(in super::super) fn owned_result_digest(
    root: &Path,
    relative: &Path,
) -> Result<[u8; 32], Error> {
    let bytes = owned_resolution_file_bytes(
        root,
        relative,
        RepositoryOperation::RepositorySnapshot,
        root,
    )
    .map_err(|_| Error::ExternalChange)?
    .ok_or(Error::ExternalChange)?;
    Ok(*blake3::hash(&bytes).as_bytes())
}

/// Stage only a previously validated/written regular canonical result into the
/// caller's private index. This does NOT persist that index or update any ref.
pub(in super::super) fn stage_result(
    repository: &git2::Repository,
    index: &mut git2::Index,
    path: &[u8],
    result: &[u8],
) -> Result<(), Error> {
    let relative = Path::new(std::str::from_utf8(path).map_err(|_| Error::RecoveryRequired)?);
    let blob = repository
        .blob(result)
        .map_err(|_| Error::RecoveryRequired)?;
    for stage in 1..=3 {
        index
            .remove(relative, stage)
            .map_err(|_| Error::RecoveryRequired)?;
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
            file_size: result.len() as u32,
            id: blob,
            flags: 0,
            flags_extended: 0,
            path: path.to_vec(),
        })
        .map_err(|_| Error::RecoveryRequired)
}
