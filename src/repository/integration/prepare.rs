//! Memory-only merge preparation and a separate, explicitly authorized import.
use super::Error;
use git2::Oid;
use std::path::Path;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in super::super) enum IntegrationDisposition {
    Equal,
    IncomingAlreadyIntegrated,
    FastForward,
    MergeRequired,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in super::super) enum IntegrationClassificationError<E> {
    Ancestry(E),
    UnrelatedHistories,
}

/// Classify exactly one ordered pair using the caller's observed graph.
pub(in super::super) fn classify_integration<E>(
    local: Oid,
    incoming: Oid,
    mut is_ancestor: impl FnMut(Oid, Oid) -> Result<bool, E>,
    mut merge_base_exists: impl FnMut(Oid, Oid) -> Result<bool, E>,
) -> Result<IntegrationDisposition, IntegrationClassificationError<E>> {
    if local == incoming {
        return Ok(IntegrationDisposition::Equal);
    }
    if is_ancestor(incoming, local).map_err(IntegrationClassificationError::Ancestry)? {
        return Ok(IntegrationDisposition::IncomingAlreadyIntegrated);
    }
    if is_ancestor(local, incoming).map_err(IntegrationClassificationError::Ancestry)? {
        return Ok(IntegrationDisposition::FastForward);
    }
    if !merge_base_exists(local, incoming).map_err(IntegrationClassificationError::Ancestry)? {
        return Err(IntegrationClassificationError::UnrelatedHistories);
    }
    Ok(IntegrationDisposition::MergeRequired)
}

const GITLINK_MODE: u32 = 0o160000;

struct PreparedMergeEntry {
    entry: git2::IndexEntry,
    // None for stored objects and gitlinks belonging to another repository.
    generated: Option<(git2::ObjectType, Vec<u8>)>,
}

pub(in super::super) struct PreparedMerge {
    entries: Vec<PreparedMergeEntry>,
}

pub(in super::super) enum MergePreparation {
    Clean(PreparedMerge),
    Conflict,
}

/// A separate handle and priority-1000 mempack prevent persistent merge writes.
pub(in super::super) fn prepare_clean_merge(
    path: &Path,
    local: Oid,
    incoming: Oid,
) -> Result<MergePreparation, Error> {
    let worker = git2::Repository::open(path).map_err(|_| Error::RecoveryRequired)?;
    let odb = worker.odb().map_err(|_| Error::RecoveryRequired)?;
    let _mempack = odb
        .add_new_mempack_backend(1000)
        .map_err(|_| Error::RecoveryRequired)?;
    prepare_clean_merge_in(&worker, path, local, incoming)
}

/// The caller MUST install a priority-1000 memory backend first. Incoming may
/// itself be an unreferenced in-memory preview commit. `path` identifies the
/// persistent ODB used only to distinguish stored from generated objects.
pub(in super::super) fn prepare_clean_merge_in(
    worker: &git2::Repository,
    path: &Path,
    local: Oid,
    incoming: Oid,
) -> Result<MergePreparation, Error> {
    let odb = worker.odb().map_err(|_| Error::RecoveryRequired)?;
    let local = worker
        .find_commit(local)
        .map_err(|_| Error::RecoveryRequired)?;
    let incoming = worker
        .find_commit(incoming)
        .map_err(|_| Error::RecoveryRequired)?;
    let index = worker
        .merge_commits(&local, &incoming, None)
        .map_err(|_| Error::RecoveryRequired)?;
    if index.has_conflicts() {
        return Ok(MergePreparation::Conflict);
    }
    let stored = git2::Repository::open(path).map_err(|_| Error::RecoveryRequired)?;
    let stored = stored.odb().map_err(|_| Error::RecoveryRequired)?;
    let entries = index
        .iter()
        .filter(|entry| entry.flags & 0x3000 == 0)
        .map(|entry| {
            if entry.mode == GITLINK_MODE || stored.exists(entry.id) {
                return Ok(PreparedMergeEntry {
                    entry,
                    generated: None,
                });
            }
            let object = odb.read(entry.id).map_err(|_| Error::RecoveryRequired)?;
            Ok(PreparedMergeEntry {
                generated: Some((object.kind(), object.data().to_vec())),
                entry,
            })
        })
        .collect::<Result<Vec<_>, Error>>()?;
    Ok(MergePreparation::Clean(PreparedMerge { entries }))
}

/// Import after the adapter revalidates its owner and frozen baseline under its
/// held lease. Callback errors are returned unchanged, before any ODB effect.
pub(in super::super) fn import_owned_tree<E: From<Error>>(
    repository: &git2::Repository,
    prepared: &PreparedMerge,
    authorize: impl FnOnce() -> Result<(), E>,
) -> Result<Oid, E> {
    authorize()?;
    import_prepared_tree(repository, prepared).map_err(E::from)
}

/// Low-level import also used by memory-backed previews. Persistent callers
/// must already hold their lease and have passed their adapter's owner gate.
pub(in super::super) fn import_prepared_tree(
    repository: &git2::Repository,
    prepared: &PreparedMerge,
) -> Result<Oid, Error> {
    let destination = repository.odb().map_err(|_| Error::RecoveryRequired)?;
    let mut index = git2::Index::new().map_err(|_| Error::RecoveryRequired)?;
    for prepared_entry in &prepared.entries {
        if let Some((kind, bytes)) = &prepared_entry.generated
            && destination
                .write(*kind, bytes)
                .map_err(|_| Error::RecoveryRequired)?
                != prepared_entry.entry.id
        {
            return Err(Error::RecoveryRequired);
        }
        index
            .add(&prepared_entry.entry)
            .map_err(|_| Error::RecoveryRequired)?;
    }
    index
        .write_tree_to(repository)
        .map_err(|_| Error::RecoveryRequired)
}

/// Read-only conflict/non-conflict stage reconstruction. Generated objects live
/// only in the temporary overlay; the returned index owns entries, not an ODB.
pub(in super::super) fn prepare_recorded_merge_index(
    repository: &git2::Repository,
    local: Oid,
    incoming: Oid,
) -> Result<git2::Index, Error> {
    let worker = git2::Repository::open(repository.path()).map_err(|_| Error::RecoveryRequired)?;
    let odb = worker.odb().map_err(|_| Error::RecoveryRequired)?;
    let _mempack = odb
        .add_new_mempack_backend(1000)
        .map_err(|_| Error::RecoveryRequired)?;
    let local = worker
        .find_commit(local)
        .map_err(|_| Error::RecoveryRequired)?;
    let incoming = worker
        .find_commit(incoming)
        .map_err(|_| Error::RecoveryRequired)?;
    worker
        .merge_commits(&local, &incoming, None)
        .map_err(|_| Error::RecoveryRequired)
}
