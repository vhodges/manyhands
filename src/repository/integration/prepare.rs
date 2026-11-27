//! Memory-only merge preparation and a separate, explicitly authorized import.
use super::{Error, validate};
use crate::{canonical, repository::closure::observe::closure_replaced_bytes};
use git2::Oid;
use std::path::Path;
use time::OffsetDateTime;

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

/// A ticket context's merge, given the primary it fetched (R-30, Part G),
/// for the ticket at `ticket`: `prepared` is `prepare_clean_merge`'s answer for
/// `local` and `incoming`, and `primary` the fetched primary of the window.
///
/// A clean merge whose ticket does not hold the closure `primary` holds is
/// given it, canonically (R-30). A conflict is settled, as the clean merge
/// that keeps `incoming`'s closure, when `settles` (the Context stage) and all
/// of these hold: the ticket is the only conflicted path; both sides hold a
/// closure, and they differ; `primary` holds none; and with `local`'s ticket
/// given `incoming`'s closure the merge is clean. Any other conflict stays.
pub(in super::super) fn keep_primary_closure(
    path: &Path,
    ticket: &Path,
    local: Oid,
    incoming: Oid,
    primary: Oid,
    settles: bool,
    prepared: MergePreparation,
) -> Result<MergePreparation, Error> {
    match prepared {
        MergePreparation::Clean(mut merge) => {
            give_primary_closure(path, ticket, primary, &mut merge)?;
            Ok(MergePreparation::Clean(merge))
        }
        MergePreparation::Conflict if settles => Ok(settle_closure_conflict(
            path, ticket, local, incoming, primary,
        )?
        .unwrap_or(MergePreparation::Conflict)),
        MergePreparation::Conflict => Ok(MergePreparation::Conflict),
    }
}

/// The ticket the commit holds at `ticket`, when it holds a canonical one.
fn ticket_in(
    repository: &git2::Repository,
    commit: Oid,
    ticket: &Path,
) -> Result<Option<canonical::Ticket>, Error> {
    Ok(match validate::primary_item(repository, commit, ticket)? {
        Some(canonical::CanonicalItem::Ticket(held)) => Some(held),
        _ => None,
    })
}

/// The closure `primary` holds for the ticket, when it holds one.
fn primary_closure(
    repository: &git2::Repository,
    primary: Oid,
    ticket: &Path,
) -> Result<Option<(OffsetDateTime, String)>, Error> {
    Ok(ticket_in(repository, primary, ticket)?.and_then(|held| held.closed_at.zip(held.closed_by)))
}

/// Whether the ticket of `head` lacks the closure `primary` holds (R-30): the
/// primary holds one, and the head's ticket holds another or none. A head
/// that holds no canonical ticket there is the validators' to refuse.
pub(in super::super) fn lacks_primary_closure(
    repository: &git2::Repository,
    head: Oid,
    primary: Oid,
    ticket: &Path,
) -> Result<bool, Error> {
    let Some(closure) = primary_closure(repository, primary, ticket)? else {
        return Ok(false);
    };
    Ok(ticket_in(repository, head, ticket)?
        .is_some_and(|held| held.closed_at.zip(held.closed_by) != Some(closure)))
}

/// The ticket of a clean merge, given the closure the fetched primary holds
/// when the result's differs (R-30).
fn give_primary_closure(
    path: &Path,
    ticket: &Path,
    primary: Oid,
    merge: &mut PreparedMerge,
) -> Result<(), Error> {
    let stored = git2::Repository::open(path).map_err(|_| Error::RecoveryRequired)?;
    let Some((closed_at, closed_by)) = primary_closure(&stored, primary, ticket)? else {
        return Ok(());
    };
    let Some(prepared) = merge
        .entries
        .iter_mut()
        .find(|prepared| prepared.entry.path == ticket_path_bytes(ticket))
    else {
        return Ok(());
    };
    let bytes = match &prepared.generated {
        Some((_, bytes)) => bytes.clone(),
        None => stored
            .find_blob(prepared.entry.id)
            .map_err(|_| Error::RecoveryRequired)?
            .content()
            .to_vec(),
    };
    // A result that is no canonical ticket is the validators' to refuse.
    let Some(result) = std::str::from_utf8(&bytes)
        .ok()
        .and_then(|text| canonical::parse_item(ticket, text).ok())
    else {
        return Ok(());
    };
    let canonical::CanonicalItem::Ticket(result) = result else {
        return Ok(());
    };
    if result.closed_at == Some(closed_at) && result.closed_by.as_deref() == Some(&closed_by) {
        return Ok(());
    }
    let replaced = closure_replaced_bytes(&bytes, closed_at, &closed_by)
        .map_err(|_| Error::RecoveryRequired)?;
    prepared.entry.id =
        Oid::hash_object(git2::ObjectType::Blob, &replaced).map_err(|_| Error::RecoveryRequired)?;
    prepared.entry.file_size =
        u32::try_from(replaced.len()).map_err(|_| Error::RecoveryRequired)?;
    prepared.generated = Some((git2::ObjectType::Blob, replaced));
    Ok(())
}

fn ticket_path_bytes(ticket: &Path) -> Vec<u8> {
    ticket.to_string_lossy().into_owned().into_bytes()
}

/// Part G: the clean merge of `local` given `incoming`'s closure, when the
/// conflict of the two is the closures of the ticket alone; none otherwise.
fn settle_closure_conflict(
    path: &Path,
    ticket: &Path,
    local: Oid,
    incoming: Oid,
    primary: Oid,
) -> Result<Option<MergePreparation>, Error> {
    let stored = git2::Repository::open(path).map_err(|_| Error::RecoveryRequired)?;
    if primary_closure(&stored, primary, ticket)?.is_some() {
        return Ok(None);
    }
    let worker = git2::Repository::open(path).map_err(|_| Error::RecoveryRequired)?;
    let odb = worker.odb().map_err(|_| Error::RecoveryRequired)?;
    let _mempack = odb
        .add_new_mempack_backend(1000)
        .map_err(|_| Error::RecoveryRequired)?;
    let local_commit = worker
        .find_commit(local)
        .map_err(|_| Error::RecoveryRequired)?;
    let incoming_commit = worker
        .find_commit(incoming)
        .map_err(|_| Error::RecoveryRequired)?;
    let index = worker
        .merge_commits(&local_commit, &incoming_commit, None)
        .map_err(|_| Error::RecoveryRequired)?;
    let conflicts = index
        .conflicts()
        .and_then(|conflicts| conflicts.collect::<Result<Vec<_>, _>>())
        .map_err(|_| Error::RecoveryRequired)?;
    let [conflict] = conflicts.as_slice() else {
        return Ok(None);
    };
    let (Some(ours), Some(theirs)) = (&conflict.our, &conflict.their) else {
        return Ok(None);
    };
    if ours.path != ticket_path_bytes(ticket) || theirs.path != ours.path {
        return Ok(None);
    }
    let read = |entry: &git2::IndexEntry| -> Result<(Vec<u8>, Option<canonical::Ticket>), Error> {
        let bytes = worker
            .find_blob(entry.id)
            .map_err(|_| Error::RecoveryRequired)?
            .content()
            .to_vec();
        let held = std::str::from_utf8(&bytes)
            .ok()
            .and_then(|text| canonical::parse_item(ticket, text).ok())
            .and_then(|item| match item {
                canonical::CanonicalItem::Ticket(held) => Some(held),
                _ => None,
            });
        Ok((bytes, held))
    };
    let ((ours_bytes, ours_ticket), (_, theirs_ticket)) = (read(ours)?, read(theirs)?);
    let (Some(ours_ticket), Some(theirs_ticket)) = (ours_ticket, theirs_ticket) else {
        return Ok(None);
    };
    let (Some(closed_at), Some(closed_by)) = (theirs_ticket.closed_at, theirs_ticket.closed_by)
    else {
        return Ok(None);
    };
    if ours_ticket.closed_at.is_none()
        || (ours_ticket.closed_at, &ours_ticket.closed_by)
            == (Some(closed_at), &Some(closed_by.clone()))
    {
        return Ok(None);
    }
    let Ok(replaced) = closure_replaced_bytes(&ours_bytes, closed_at, &closed_by) else {
        return Ok(None);
    };
    let mode = if ours.mode == 0o100755 {
        git2::FileMode::BlobExecutable
    } else {
        git2::FileMode::Blob
    };
    let blob = worker
        .blob(&replaced)
        .map_err(|_| Error::RecoveryRequired)?;
    let tree = git2::build::TreeUpdateBuilder::new()
        .upsert(ticket, blob, mode)
        .create_updated(
            &worker,
            &local_commit.tree().map_err(|_| Error::RecoveryRequired)?,
        )
        .and_then(|tree| worker.find_tree(tree))
        .map_err(|_| Error::RecoveryRequired)?;
    // A child of `local`, so that its merge base with `incoming` is the same.
    let signature =
        git2::Signature::new("Manyhands", "manyhands@localhost", &git2::Time::new(0, 0))
            .map_err(|_| Error::RecoveryRequired)?;
    let settled = worker
        .commit(
            None,
            &signature,
            &signature,
            "settled",
            &tree,
            &[&local_commit],
        )
        .map_err(|_| Error::RecoveryRequired)?;
    match prepare_clean_merge_in(&worker, path, settled, incoming)? {
        clean @ MergePreparation::Clean(_) => Ok(Some(clean)),
        MergePreparation::Conflict => Ok(None),
    }
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
