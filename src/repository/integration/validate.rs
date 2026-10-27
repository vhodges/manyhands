//! Canonical validation from immutable Git sides, never mutable caller paths.
use super::Error;
use crate::canonical;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConflictEligibility {
    EligibleCanonical,
    ExternalResolutionRequired,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in super::super) enum ConflictEntryKind {
    Document,
    Ticket,
    Comment,
    Noncanonical,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in super::super) enum ConflictStructure {
    RegularUtf8SamePath,
    Binary,
    Executable,
    Symlink,
    Rename,
    Delete,
    IdentityChanged,
    IncompatibleClosures,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in super::super) struct ConflictCandidate {
    pub(in super::super) kind: ConflictEntryKind,
    pub(in super::super) structure: ConflictStructure,
}

/// One unsupported entry makes the entire set external-only.
pub(in super::super) fn conflict_eligibility(entries: &[ConflictCandidate]) -> ConflictEligibility {
    if !entries.is_empty()
        && entries.iter().all(|entry| {
            matches!(
                entry.kind,
                ConflictEntryKind::Document
                    | ConflictEntryKind::Ticket
                    | ConflictEntryKind::Comment
            ) && entry.structure == ConflictStructure::RegularUtf8SamePath
        })
    {
        ConflictEligibility::EligibleCanonical
    } else {
        ConflictEligibility::ExternalResolutionRequired
    }
}

pub(in super::super) fn classify_conflict_entry(
    repository: &git2::Repository,
    conflict: &git2::IndexConflict,
) -> ConflictCandidate {
    let entries = [
        conflict.ancestor.as_ref(),
        conflict.our.as_ref(),
        conflict.their.as_ref(),
    ];
    let Some(path) = entries.iter().flatten().next().map(|entry| &entry.path) else {
        return ConflictCandidate {
            kind: ConflictEntryKind::Noncanonical,
            structure: ConflictStructure::Delete,
        };
    };
    if entries.iter().any(Option::is_none) {
        return ConflictCandidate {
            kind: ConflictEntryKind::Noncanonical,
            structure: ConflictStructure::Delete,
        };
    }
    if entries.iter().flatten().any(|entry| entry.path != *path) {
        return ConflictCandidate {
            kind: ConflictEntryKind::Noncanonical,
            structure: ConflictStructure::Rename,
        };
    }
    if entries.iter().flatten().any(|entry| entry.mode != 0o100644) {
        let structure = if entries.iter().flatten().any(|entry| entry.mode == 0o120000) {
            ConflictStructure::Symlink
        } else {
            ConflictStructure::Executable
        };
        return ConflictCandidate {
            kind: ConflictEntryKind::Noncanonical,
            structure,
        };
    }
    let Ok(path) = std::str::from_utf8(path) else {
        return ConflictCandidate {
            kind: ConflictEntryKind::Noncanonical,
            structure: ConflictStructure::Binary,
        };
    };
    if Path::new(path)
        .components()
        .any(|part| !matches!(part, std::path::Component::Normal(_)))
    {
        return ConflictCandidate {
            kind: ConflictEntryKind::Noncanonical,
            structure: ConflictStructure::Rename,
        };
    }
    let parsed = entries
        .iter()
        .flatten()
        .map(|entry| {
            repository.find_blob(entry.id).ok().and_then(|blob| {
                std::str::from_utf8(blob.content())
                    .ok()
                    .and_then(|text| canonical::parse_item(Path::new(path), text).ok())
            })
        })
        .collect::<Option<Vec<_>>>();
    let Some(parsed) = parsed else {
        return ConflictCandidate {
            kind: ConflictEntryKind::Noncanonical,
            structure: ConflictStructure::Binary,
        };
    };
    let Some(first) = parsed.first() else {
        return ConflictCandidate {
            kind: ConflictEntryKind::Noncanonical,
            structure: ConflictStructure::Delete,
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
        return ConflictCandidate {
            kind: ConflictEntryKind::Noncanonical,
            structure: ConflictStructure::IdentityChanged,
        };
    }
    let kind = match first {
        canonical::CanonicalItem::Document(_) => ConflictEntryKind::Document,
        canonical::CanonicalItem::Ticket(_) => ConflictEntryKind::Ticket,
        canonical::CanonicalItem::Comment(_) => ConflictEntryKind::Comment,
    };
    let mut closures = parsed.iter().filter_map(|item| match item {
        canonical::CanonicalItem::Ticket(ticket) => {
            ticket.closed_at.map(|at| (at, ticket.closed_by.as_deref()))
        }
        _ => None,
    });
    if let Some(first_closure) = closures.next()
        && closures.any(|closure| closure != first_closure)
    {
        return ConflictCandidate {
            kind,
            structure: ConflictStructure::IncompatibleClosures,
        };
    }
    ConflictCandidate {
        kind,
        structure: ConflictStructure::RegularUtf8SamePath,
    }
}

pub(in super::super) fn canonical_item_id(item: &canonical::CanonicalItem) -> canonical::ItemId {
    match item {
        canonical::CanonicalItem::Document(value) => value.id.clone(),
        canonical::CanonicalItem::Ticket(value) => value.id.clone(),
        canonical::CanonicalItem::Comment(value) => value.id.clone(),
    }
}

pub(in super::super) fn preserves_item_invariants(
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
                && old.created_by == new.created_by
                && old.unknown.get("created_by") == new.unknown.get("created_by")
                && old.unknown.get("author") == new.unknown.get("author")
        }
        _ => false,
    }
}

/// All recorded sides constrain the result, including immutable fields retained
/// only in the ancestor. The adapter first binds this raw path and these OIDs to
/// its capability; this function is content validation, not write authorization.
pub(in super::super) fn validates_resolution_sides(
    repository: &git2::Repository,
    path: &[u8],
    side_ids: [Option<git2::Oid>; 3],
    bytes: &[u8],
) -> Result<bool, Error> {
    let Ok(text) = std::str::from_utf8(bytes) else {
        return Ok(false);
    };
    let path = Path::new(std::str::from_utf8(path).unwrap_or_default());
    let Ok(item) = canonical::parse_item(path, text) else {
        return Ok(false);
    };
    if side_ids.iter().all(Option::is_none) {
        return Ok(false);
    }
    for side_oid in side_ids.into_iter().flatten() {
        let blob = repository
            .find_blob(side_oid)
            .map_err(|_| Error::RecoveryRequired)?;
        let Ok(side_text) = std::str::from_utf8(blob.content()) else {
            return Ok(false);
        };
        let Ok(expected) = canonical::parse_item(path, side_text) else {
            return Ok(false);
        };
        if canonical_item_id(&item) != canonical_item_id(&expected)
            || !preserves_item_invariants(&expected, &item)
        {
            return Ok(false);
        }
    }
    Ok(true)
}

/// Borrowed, capability-checked input. No request or source persistence occurs.
pub(in super::super) struct ResolutionSource<'a> {
    pub(in super::super) path: &'a [u8],
    pub(in super::super) sides: [Option<git2::Oid>; 3],
    pub(in super::super) bytes: &'a [u8],
}

pub(in super::super) fn validates_prospective_context(
    repository: &git2::Repository,
    replacements: &[ResolutionSource<'_>],
) -> Result<bool, Error> {
    let mut sources = BTreeMap::<Vec<u8>, Vec<u8>>::new();
    let index = repository.index().map_err(|_| Error::RecoveryRequired)?;
    for entry in index.iter() {
        if entry.flags >> 12 & 3 != 0 {
            continue;
        }
        let Ok(path) = std::str::from_utf8(&entry.path) else {
            continue;
        };
        let Ok(blob) = repository.find_blob(entry.id) else {
            return Err(Error::RecoveryRequired);
        };
        if let Ok(text) = std::str::from_utf8(blob.content())
            && canonical::parse_item(Path::new(path), text).is_ok()
        {
            sources.insert(entry.path.clone(), blob.content().to_vec());
        }
    }
    let mut baseline = sources.clone();
    for source in replacements {
        let [base, local, incoming] = source.sides;
        if let Some(oid) = local.or(incoming).or(base) {
            let blob = repository
                .find_blob(oid)
                .map_err(|_| Error::RecoveryRequired)?;
            baseline.insert(source.path.to_vec(), blob.content().to_vec());
        }
    }
    let mut affected = BTreeSet::new();
    for source in replacements {
        let Some(item) = std::str::from_utf8(source.path).ok().and_then(|path| {
            std::str::from_utf8(source.bytes)
                .ok()
                .and_then(|text| canonical::parse_item(Path::new(path), text).ok())
        }) else {
            return Ok(false);
        };
        affected.insert(canonical_item_id(&item));
        sources.insert(source.path.to_vec(), source.bytes.to_vec());
    }
    Ok(validates_context_sources(&baseline, &sources, affected))
}

pub(in super::super) fn validates_context_sources(
    baseline: &BTreeMap<Vec<u8>, Vec<u8>>,
    sources: &BTreeMap<Vec<u8>, Vec<u8>>,
    mut affected: BTreeSet<canonical::ItemId>,
) -> bool {
    let context_sources = |sources: &BTreeMap<Vec<u8>, Vec<u8>>| {
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
    // Close over item/thread dependencies in both directions, including invalid
    // candidates omitted from ValidatedContext.items.
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
        let path = problem.path.to_string_lossy().as_bytes().to_vec();
        baseline.get(&path) == sources.get(&path)
            && baseline_context.problems.contains(problem)
            && !items.iter().any(|(path, item)| {
                path == &problem.path && affected.contains(&canonical_item_id(item))
            })
    })
}

/// The non-conflicting stage-zero portion is wholly determined by frozen parents.
/// Unrelated staged entries cannot be legitimized by matching conflict tokens.
pub(in super::super) fn prepared_merge_index_matches_index(
    expected: &git2::Index,
    actual: &git2::Index,
) -> Result<bool, Error> {
    let conflicts = expected
        .conflicts()
        .map_err(|_| Error::RecoveryRequired)?
        .map(|conflict| {
            let conflict = conflict.map_err(|_| Error::RecoveryRequired)?;
            conflict
                .our
                .or(conflict.their)
                .or(conflict.ancestor)
                .map(|entry| entry.path)
                .ok_or(Error::RecoveryRequired)
        })
        .collect::<Result<BTreeSet<_>, _>>()?;
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

/// Ephemeral ODB evidence retains shape/bytes even when canonical parsing fails.
pub(in super::super) struct CanonicalTreeSnapshot {
    pub(in super::super) sources: BTreeMap<Vec<u8>, Vec<u8>>,
    pub(in super::super) entries: BTreeMap<Vec<u8>, (git2::Oid, i32, Option<git2::ObjectType>)>,
}

pub(in super::super) fn canonical_tree_sources(
    repository: &git2::Repository,
    tree: &git2::Tree<'_>,
) -> Result<CanonicalTreeSnapshot, Error> {
    let mut sources = BTreeMap::new();
    let mut entries = BTreeMap::new();
    let mut failed = false;
    tree.walk(git2::TreeWalkMode::PreOrder, |parent, entry| {
        let mut path = parent.as_bytes().to_vec();
        path.extend_from_slice(entry.name_bytes());
        let path_text = std::str::from_utf8(&path).ok();
        let canonical_path = (path.ends_with(b".md")
            && (path.starts_with(b"docs/")
                || path.starts_with(b".manyhands/tickets/")
                || path.starts_with(b".manyhands/comments/")))
            || path_text.is_some_and(|path| is_canonical_tree_source_path(Path::new(path)));
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
                            .is_some_and(|text| is_canonical_tree_source(Path::new(path), text))
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
    .map_err(|_| Error::RecoveryRequired)?;
    if failed {
        return Err(Error::RecoveryRequired);
    }
    Ok(CanonicalTreeSnapshot { sources, entries })
}

/// Called only after omitted undecodable sources are proved byte/shape-identical.
fn canonical_context_tree_sources(snapshot: &CanonicalTreeSnapshot) -> BTreeMap<Vec<u8>, Vec<u8>> {
    snapshot
        .sources
        .iter()
        .filter_map(|(path, bytes)| {
            let path_text = std::str::from_utf8(path).ok()?;
            let text = std::str::from_utf8(bytes).ok()?;
            is_canonical_tree_source(Path::new(path_text), text)
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
    // Include malformed declared-managed items so new diagnostics are rejected.
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

/// Canonical content proof for an external whole-merge commit. The adapter must
/// separately prove exact ordered parents, clean index/worktree and artifact
/// ownership before accepting it. Subject or authoring metadata is not authority.
pub(in super::super) fn validates_external_integration(
    repository: &git2::Repository,
    local: git2::Oid,
    incoming: git2::Oid,
    candidate: git2::Oid,
) -> Result<bool, Error> {
    let local = repository
        .find_commit(local)
        .map_err(|_| Error::RecoveryRequired)?;
    let incoming = repository
        .find_commit(incoming)
        .map_err(|_| Error::RecoveryRequired)?;
    let base = repository
        .merge_base(local.id(), incoming.id())
        .and_then(|oid| repository.find_commit(oid))
        .map_err(|_| Error::RecoveryRequired)?;
    let result = repository
        .find_commit(candidate)
        .map_err(|_| Error::RecoveryRequired)?;
    let result_tree = result.tree().map_err(|_| Error::RecoveryRequired)?;
    let sides = [base.tree(), local.tree(), incoming.tree()]
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| Error::RecoveryRequired)?;
    let baseline = canonical_tree_sources(repository, &sides[1])?;
    let prospective = canonical_tree_sources(repository, &result_tree)?;
    for (path, shape) in &prospective.entries {
        if baseline.entries.get(path) == Some(shape) {
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
    let mut changed = BTreeSet::new();
    for side in &sides {
        let diff = repository
            .diff_tree_to_tree(Some(side), Some(&result_tree), None)
            .map_err(|_| Error::RecoveryRequired)?;
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
    let mut affected = BTreeSet::new();
    for path in changed {
        let result_item = parse(&result_tree, &path);
        for side in &sides {
            if let Some(old) = parse(side, &path) {
                let Some(new) = &result_item else {
                    return Ok(false);
                };
                if canonical_item_id(&old) != canonical_item_id(new)
                    || !preserves_item_invariants(&old, new)
                {
                    return Ok(false);
                }
                affected.insert(canonical_item_id(&old));
            }
        }
        if let Some(item) = result_item {
            affected.insert(canonical_item_id(&item));
        }
    }
    Ok(validates_context_sources(
        &canonical_context_tree_sources(&baseline),
        &canonical_context_tree_sources(&prospective),
        affected,
    ))
}
