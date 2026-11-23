use crate::{
    canonical::{self, CanonicalItem, ItemId},
    repository::AuthoringKind,
};
use std::{collections::BTreeMap, path::Path};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in super::super) enum PreservationProof {
    Ancestor,
    Folded,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in super::super) enum ProofRefusal {
    InvalidPrimaryItem,
    TicketOpen,
    MissingObject,
    ShallowHistory,
    UnrelatedHistory,
    MultipleMergeBases,
    ContentMismatch,
}

pub(in super::super) fn preservation(
    repository: &git2::Repository,
    local: git2::Oid,
    primary: git2::Oid,
    kind: AuthoringKind,
    id: &ItemId,
) -> Result<PreservationProof, ProofRefusal> {
    use ProofRefusal::*;
    if repository.is_shallow() {
        return Err(ShallowHistory);
    }
    let local_tree = entries(repository, local, true)?;
    let primary_tree = entries(repository, primary, true)?;
    completed_item(repository, &primary_tree, kind, id)?;
    if local == primary
        || repository
            .graph_descendant_of(primary, local)
            .map_err(|_| MissingObject)?
    {
        return Ok(PreservationProof::Ancestor);
    }
    let bases = repository.merge_bases(local, primary).map_err(|error| {
        if error.code() == git2::ErrorCode::NotFound {
            UnrelatedHistory
        } else {
            MissingObject
        }
    })?;
    let base = match bases.as_ref() {
        [base] => *base,
        [] => return Err(UnrelatedHistory),
        _ => return Err(MultipleMergeBases),
    };
    let base_tree = entries(repository, base, true)?;
    let exceptional = format!(".manyhands/tickets/{id}/ticket.md").into_bytes();
    // Compare the complete raw-path delta, not a pathspec or rename-similarity
    // diff. Flattening leaves (including empty trees) turns replacements and renames into
    // their constituent deletions/additions; inherited untouched paths are free
    // to have advanced on primary.
    for path in base_tree.keys().chain(local_tree.keys()) {
        let local_entry = local_tree.get(path);
        if base_tree.get(path) == local_entry || primary_tree.get(path) == local_entry {
            continue;
        }
        let exception = kind == AuthoringKind::Ticket
            && *path == exceptional
            && local_entry
                .zip(primary_tree.get(path))
                .is_some_and(|(a, b)| {
                    a.mode == b.mode
                        && a.regular()
                        && b.regular()
                        && older_open_ticket(repository, path, a.oid, b.oid, id)
                });
        if !exception {
            return Err(ContentMismatch);
        }
    }
    Ok(PreservationProof::Folded)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Entry {
    oid: git2::Oid,
    mode: i32,
    kind: Option<git2::ObjectType>,
}
impl Entry {
    fn regular(self) -> bool {
        self.kind == Some(git2::ObjectType::Blob) && matches!(self.mode, 0o100644 | 0o100755)
    }
}
type Entries = BTreeMap<Vec<u8>, Entry>;

fn entries(
    repository: &git2::Repository,
    tip: git2::Oid,
    verify_objects: bool,
) -> Result<Entries, ProofRefusal> {
    let tree = repository
        .find_commit(tip)
        .map_err(|_| ProofRefusal::MissingObject)?
        .tree_id();
    let mut out = Entries::new();
    let mut pending = vec![(Vec::new(), tree)];
    while let Some((prefix, oid)) = pending.pop() {
        let tree = repository
            .find_tree(oid)
            .map_err(|_| ProofRefusal::MissingObject)?;
        for entry in &tree {
            let mut path = prefix.clone();
            path.extend_from_slice(entry.name_bytes());
            let value = Entry {
                oid: entry.id(),
                mode: entry.filemode_raw(),
                kind: entry.kind(),
            };
            if entry.kind() == Some(git2::ObjectType::Tree) {
                let tree = repository
                    .find_tree(entry.id())
                    .map_err(|_| ProofRefusal::MissingObject)?;
                if tree.is_empty() {
                    out.insert(path, value);
                } else {
                    path.push(b'/');
                    pending.push((path, entry.id()));
                }
            } else {
                if verify_objects {
                    entry
                        .to_object(repository)
                        .map_err(|_| ProofRefusal::MissingObject)?;
                }
                out.insert(path, value);
            }
        }
    }
    Ok(out)
}

/// Positive completion only, independently useful for dirty/unmatched tickets.
/// All reads are committed Git objects; no discovery scanner or worktree read.
/// For documents this establishes only presence: callers must also prove preservation.
pub(in super::super) fn completion(
    repository: &git2::Repository,
    primary: git2::Oid,
    kind: AuthoringKind,
    id: &ItemId,
) -> Result<(), ProofRefusal> {
    completed_item(repository, &entries(repository, primary, false)?, kind, id)
}

fn completed_item(
    repository: &git2::Repository,
    entries: &Entries,
    kind: AuthoringKind,
    id: &ItemId,
) -> Result<(), ProofRefusal> {
    use ProofRefusal::*;
    let ticket_path = format!(".manyhands/tickets/{id}/ticket.md");
    let id_text = id.to_string();
    let mut found = None;
    for (path, entry) in entries {
        let Ok(path) = std::str::from_utf8(path) else {
            continue;
        };
        if !(path.starts_with("docs/")
            || path.starts_with(".manyhands/tickets/")
            || path.starts_with(".manyhands/comments/"))
            || !path.ends_with(".md")
        {
            continue;
        }
        let blob = repository
            .find_blob(entry.oid)
            .map_err(|_| InvalidPrimaryItem)?;
        let source = std::str::from_utf8(blob.content()).ok();
        let same_id = source
            .and_then(canonical::front_matter)
            .and_then(|fm| fm.get("id").and_then(|id| id.as_str()).map(str::to_owned))
            .as_deref()
            == Some(id_text.as_str());
        if !(same_id || kind == AuthoringKind::Ticket && path == ticket_path) {
            continue;
        }
        if found.is_some() || !entry.regular() {
            return Err(InvalidPrimaryItem);
        }
        let item = source
            .and_then(|s| canonical::parse_item(Path::new(path), s).ok())
            .ok_or(InvalidPrimaryItem)?;
        match (&item, kind) {
            (CanonicalItem::Ticket(ticket), AuthoringKind::Ticket) if ticket.id == *id => {}
            (CanonicalItem::Document(document), AuthoringKind::Document) if document.id == *id => {}
            _ => return Err(InvalidPrimaryItem),
        }
        found = Some(item);
    }
    match found.ok_or(InvalidPrimaryItem)? {
        CanonicalItem::Ticket(ticket)
            if ticket.closed_at.is_none() || ticket.closed_by.is_none() =>
        {
            Err(TicketOpen)
        }
        _ => Ok(()),
    }
}

fn older_open_ticket(
    repository: &git2::Repository,
    path: &[u8],
    local: git2::Oid,
    primary: git2::Oid,
    id: &ItemId,
) -> bool {
    let normalize = |oid| -> Option<canonical::Ticket> {
        let blob = repository.find_blob(oid).ok()?;
        let path = Path::new(std::str::from_utf8(path).ok()?);
        let item = canonical::parse_item(path, std::str::from_utf8(blob.content()).ok()?).ok()?;
        let normalized = canonical::serialize_item(&item).ok()?;
        match canonical::parse_item(path, &normalized).ok()? {
            CanonicalItem::Ticket(ticket) if ticket.id == *id => Some(ticket),
            _ => None,
        }
    };
    let Some(local) = normalize(local) else {
        return false;
    };
    let Some(mut primary) = normalize(primary) else {
        return false;
    };
    if local.closed_at.is_some()
        || local.closed_by.is_some()
        || primary.closed_at.is_none()
        || primary.closed_by.is_none()
    {
        return false;
    }
    primary.closed_at = None;
    primary.closed_by = None;
    local == primary
}
