//! When an item's done state reached the primary: the arrival record.
//!
//! A move of the local primary brings items to it: a ticket whose closure the
//! new tip holds and the old one did not, a document whose content it holds
//! anew. `arrivals` decides which, and at which first-parent commit of the
//! move's range, for every move by one rule. `record` keeps the result in the
//! registry, one row per item, and `record_after_move` does both once a move
//! has succeeded. The time is for display: a failure to find or record it is
//! dropped, and never changes the answer of the verb that moved the primary.

use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
};

use git2::{Commit, Delta, DiffOptions, Oid, Repository};
use rusqlite::{OptionalExtension, Transaction, params};
use time::OffsetDateTime;

use super::{AuthoringKind, OperationId, RepositoryError, RepositoryService, remote::state};
use crate::canonical::{self, CanonicalItem, ItemId, ItemKind};

/// An item the primary's move brought: the first-parent commit of the range
/// that last changed its file, and that commit's committer time.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Arrival {
    pub kind: AuthoringKind,
    pub item_id: ItemId,
    pub commit: Oid,
    pub merged_at: OffsetDateTime,
}

/// What a move did to the items: those that arrived, ordered by kind name and
/// ID, and the managed documents it removed, whose rows are to go.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct Arrivals {
    pub arrived: Vec<Arrival>,
    pub deleted: Vec<ItemId>,
}

fn kind_name(kind: AuthoringKind) -> &'static str {
    match kind {
        AuthoringKind::Ticket => "ticket",
        AuthoringKind::Document => "document",
    }
}

/// What a candidate's path is, before the walk finds its commit.
struct Candidate {
    kind: AuthoringKind,
    item_id: ItemId,
}

/// The items a move from `before` to `after` brought.
///
/// Nothing when `before` is `after` or does not lead to it. The candidates
/// are the tickets whose copy at `after` has a `closed_at` that the copy at
/// `before` lacks or has otherwise, and the managed documents whose file is
/// added or modified at `after`; a document deleted without another path
/// holding its ID is reported as deleted. Each candidate is given the newest
/// first-parent commit of the range that changed its path, by one walk that
/// diffs each commit against its first parent, restricted to the paths not yet
/// given a commit, and stops when none is left. A candidate no commit of the
/// walk changed, whose change came only through a commit `before` holds, gets
/// no arrival.
pub(super) fn arrivals(
    repository: &Repository,
    before: Oid,
    after: Oid,
) -> Result<Arrivals, git2::Error> {
    if before == after || !repository.graph_descendant_of(after, before)? {
        return Ok(Arrivals::default());
    }
    let before_tree = repository.find_commit(before)?.tree()?;
    let after_tree = repository.find_commit(after)?.tree()?;
    let diff = repository.diff_tree_to_tree(Some(&before_tree), Some(&after_tree), None)?;

    let mut candidates: BTreeMap<PathBuf, Candidate> = BTreeMap::new();
    let mut present_documents = BTreeSet::new();
    let mut removed_documents = Vec::new();
    for delta in diff.deltas() {
        let status = delta.status();
        match status {
            Delta::Added | Delta::Modified => {
                let new = delta.new_file();
                let Some(path) = new.path() else { continue };
                let Ok(kind) = canonical::item_path_kind(path) else {
                    continue;
                };
                if !matches!(
                    new.mode(),
                    git2::FileMode::Blob | git2::FileMode::BlobExecutable
                ) {
                    continue;
                }
                let Some(CanonicalItem::Ticket(after_ticket)) = (kind == ItemKind::Ticket)
                    .then(|| parse(repository, path, new.id()))
                    .flatten()
                else {
                    if kind == ItemKind::Document
                        && let Some(CanonicalItem::Document(document)) =
                            parse(repository, path, new.id())
                    {
                        present_documents.insert(document.id.clone());
                        candidates.insert(
                            path.to_owned(),
                            Candidate {
                                kind: AuthoringKind::Document,
                                item_id: document.id,
                            },
                        );
                    }
                    continue;
                };
                let Some(closed_at) = after_ticket.closed_at else {
                    continue;
                };
                let before_closed_at = (status == Delta::Modified)
                    .then(|| parse(repository, path, delta.old_file().id()))
                    .flatten()
                    .and_then(|item| match item {
                        CanonicalItem::Ticket(ticket) => ticket.closed_at,
                        _ => None,
                    });
                if before_closed_at != Some(closed_at) {
                    candidates.insert(
                        path.to_owned(),
                        Candidate {
                            kind: AuthoringKind::Ticket,
                            item_id: after_ticket.id,
                        },
                    );
                }
            }
            Delta::Deleted => {
                let old = delta.old_file();
                let Some(path) = old.path() else { continue };
                if canonical::item_path_kind(path) == Ok(ItemKind::Document)
                    && let Some(CanonicalItem::Document(document)) =
                        parse(repository, path, old.id())
                {
                    removed_documents.push(document.id);
                }
            }
            _ => {}
        }
    }
    let deleted: BTreeSet<ItemId> = removed_documents
        .into_iter()
        .filter(|id| !present_documents.contains(id))
        .collect();

    let mut arrived: BTreeMap<(&'static str, ItemId), Arrival> = BTreeMap::new();
    if !candidates.is_empty() {
        let mut walk = repository.revwalk()?;
        walk.simplify_first_parent()?;
        walk.push(after)?;
        walk.hide(before)?;
        for oid in walk {
            let commit = repository.find_commit(oid?)?;
            let changed = changed_candidate_paths(repository, &commit, &candidates)?;
            if changed.is_empty() {
                continue;
            }
            let merged_at = commit_time(&commit)?;
            for path in changed {
                let Some(candidate) = candidates.remove(&path) else {
                    continue;
                };
                arrived
                    .entry((kind_name(candidate.kind), candidate.item_id.clone()))
                    .or_insert(Arrival {
                        kind: candidate.kind,
                        item_id: candidate.item_id,
                        commit: commit.id(),
                        merged_at,
                    });
            }
            if candidates.is_empty() {
                break;
            }
        }
    }
    Ok(Arrivals {
        arrived: arrived.into_values().collect(),
        deleted: deleted.into_iter().collect(),
    })
}

fn parse(repository: &Repository, path: &Path, blob: Oid) -> Option<CanonicalItem> {
    let blob = repository.find_blob(blob).ok()?;
    canonical::parse_item(path, std::str::from_utf8(blob.content()).ok()?).ok()
}

fn commit_time(commit: &Commit<'_>) -> Result<OffsetDateTime, git2::Error> {
    OffsetDateTime::from_unix_timestamp(commit.time().seconds())
        .map_err(|error| git2::Error::from_str(&error.to_string()))
}

#[cfg(test)]
thread_local! { static DIFFED: std::cell::Cell<usize> = const { std::cell::Cell::new(0) }; }

/// How many commits this thread's walks have diffed since the last call.
#[cfg(test)]
pub(super) fn take_diffed() -> usize {
    DIFFED.with(|diffed| diffed.replace(0))
}

/// The candidate paths `commit` changed against its first parent, or against
/// the empty tree for a root, by a literal pathspec of the candidates not yet
/// given a commit.
fn changed_candidate_paths(
    repository: &Repository,
    commit: &Commit<'_>,
    candidates: &BTreeMap<PathBuf, Candidate>,
) -> Result<Vec<PathBuf>, git2::Error> {
    let mut options = DiffOptions::new();
    options.disable_pathspec_match(true);
    for path in candidates.keys() {
        options.pathspec(path);
    }
    #[cfg(test)]
    DIFFED.with(|diffed| diffed.set(diffed.get() + 1));
    let parent_tree = (commit.parent_count() > 0)
        .then(|| commit.parent(0).and_then(|parent| parent.tree()))
        .transpose()?;
    let diff = repository.diff_tree_to_tree(
        parent_tree.as_ref(),
        Some(&commit.tree()?),
        Some(&mut options),
    )?;
    Ok(diff
        .deltas()
        .flat_map(|delta| [delta.new_file().path(), delta.old_file().path()])
        .flatten()
        .filter(|path| candidates.contains_key(*path))
        .map(Path::to_owned)
        .collect())
}

#[cfg(test)]
thread_local! { static FAIL_RECORD: std::cell::Cell<bool> = const { std::cell::Cell::new(false) }; }

/// Makes this thread's `record` fail, or work again.
#[cfg(test)]
pub(super) fn fail_record_for_test(on: bool) {
    FAIL_RECORD.with(|fail| fail.set(on));
}

/// Writes the arrivals in the caller's transaction: a row for each arrived
/// item, replacing the item's earlier one unless that row's commit is not an
/// ancestor of the new one (two sites that recorded one item out of order),
/// and the deletion of each deleted document's row. An arrival whose time the
/// table cannot hold is not recorded.
pub(super) fn record(
    tx: &Transaction<'_>,
    repository: &Repository,
    repository_id: i64,
    operation: &OperationId,
    arrivals: &Arrivals,
) -> Result<(), RepositoryError> {
    #[cfg(test)]
    if FAIL_RECORD.with(std::cell::Cell::get) {
        return Err(state::recovery_required());
    }
    for arrival in &arrivals.arrived {
        let seconds = arrival.merged_at.unix_timestamp();
        if !(0..=253_402_300_799).contains(&seconds) {
            continue;
        }
        let kind = kind_name(arrival.kind);
        let item = arrival.item_id.to_string();
        let held: Option<String> = tx
            .query_row(
                "SELECT commit_oid FROM item_merges
                  WHERE repository_id = ?1 AND kind = ?2 AND item_ulid = ?3",
                params![repository_id, kind, item],
                |row| row.get(0),
            )
            .optional()
            .map_err(RepositoryError::sqlite)?;
        let newer_is_held = held
            .and_then(|held| Oid::from_str(&held).ok())
            .is_some_and(|held| {
                held != arrival.commit
                    && !repository
                        .graph_descendant_of(arrival.commit, held)
                        .unwrap_or(true)
            });
        if newer_is_held {
            continue;
        }
        tx.execute(
            "INSERT INTO item_merges
                 (repository_id, kind, item_ulid, commit_oid, merged_at, operation_ulid)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(repository_id, kind, item_ulid) DO UPDATE SET
                 commit_oid = excluded.commit_oid, merged_at = excluded.merged_at,
                 operation_ulid = excluded.operation_ulid",
            params![
                repository_id,
                kind,
                item,
                arrival.commit.to_string(),
                seconds,
                operation.to_string()
            ],
        )
        .map_err(RepositoryError::sqlite)?;
    }
    for id in &arrivals.deleted {
        tx.execute(
            "DELETE FROM item_merges
              WHERE repository_id = ?1 AND kind = 'document' AND item_ulid = ?2",
            params![repository_id, id.to_string()],
        )
        .map_err(RepositoryError::sqlite)?;
    }
    Ok(())
}

/// Finds and records what a move of the local primary from `before` to
/// `after` brought, once the move has succeeded and outside its own
/// transaction. Any failure is dropped: the move stands, and an item without
/// a row only reads without a time.
pub(super) fn record_after_move(
    service: &RepositoryService,
    root: &Path,
    operation: &OperationId,
    before: Oid,
    after: Oid,
) {
    let _ = (|| -> Result<(), ()> {
        let repository = Repository::open(root).map_err(|_| ())?;
        let arrivals = arrivals(&repository, before, after).map_err(|_| ())?;
        if arrivals.arrived.is_empty() && arrivals.deleted.is_empty() {
            return Ok(());
        }
        state::with_transaction(service, root, |tx, id| {
            record(tx, &repository, id, operation, &arrivals)
        })
        .map_err(|_| ())
    })();
}

#[cfg(test)]
#[path = "item_merges_tests.rs"]
mod tests;
