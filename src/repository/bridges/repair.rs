//! Repair and adoption: a nonconforming file replaced by a corrected
//! complete source, in an editing context, through the ordinary
//! checkpoint.
//!
//! **Primary is never written.** The file that is not an item stays on
//! primary as it is, a visible problem, until the repaired item's
//! context is merged. The corrected source is written and committed in
//! the editing context of the item it makes.
//!
//! **The ID is the caller's.** The corrected source carries the item's
//! ID, and that ID is the item's from then on. Nothing here makes one:
//! the same source gives the same ID, the same branch and the same
//! worktree on every call, whatever was lost in between.
//!
//! - *Adoption.* A file that holds no valid ID is given the source's.
//! - *Existing identity.* A file that holds a valid ID keeps it: a
//!   source with another is `NotRepairable`.
//! - *Ambiguous identity.* An ID another file holds, on primary or in
//!   the context that ID names, is `AmbiguousIdentity`. Nothing is
//!   written and no context is made.
//! - *Tickets.* A ticket's path holds its ID, so the source's ID is the
//!   directory's name. A directory that is no ULID is `NotRepairable`.
//! - *Closure.* A ticket's `closed_at` and `closed_by` are carried as
//!   they are, or stay absent: a repair neither closes nor reopens.
//! - *Not repaired here.* A managed file outside a canonical path, and a
//!   file whose kind is another's than its path's.
//!
//! **What the request expects.** `expected` is the digest of the
//! nonconforming file as the caller saw it on primary. The first call of
//! an operation finds primary's file to be exactly that, and asks the
//! rules above of it. The file in the item's context is then that file,
//! which a context cut from primary's head holds when the file is
//! committed there, or already the corrected source: an earlier call
//! that wrote and did not commit left it, under this operation or one
//! that was lost with the index. Anything else is `ExternalChange`.
//!
//! **The path is the file's own.** No link is followed to the file, on
//! primary or in the context: it is read and written by the walk every
//! owned write uses. And the path is the one the file's directories list
//! it under, as `create_folder` reads a name: a path that a file system
//! which folds case would take for the file's is not repaired through.
//!
//! **Nothing of the file is said.** No error names what a file or a
//! source holds: a parser's message can repeat the text it read.

use std::path::{Path, PathBuf};

use git2::Repository;

use crate::canonical::{self, CanonicalItem, ItemId, ItemKind};

use super::super::{
    AuthoringKind, AuthoringTarget, CallEffects, CheckpointSpec, ContextIntent,
    ContextProvisionOutcome, ExpectedPathObservation, ExternalChangeDiagnostic,
    ExternalChangeExpectation, FailurePoint, LocalCheckpoint, OperationId, RepositoryError,
    RepositoryErrorKind, RepositoryOperation, RepositoryService, SaveOutcome,
    authoring_kind_segment, authoring_write_target, bridges::folder::is_listed_file,
    canonical_repository_root, checkpoint_after_refresh, collect_canonical_sources,
    ensure_replayed_owned_destination, ensure_safe_owned_parent, item_context,
    observe_owned_regular_file, owned_file_bytes, recovery::LocalStep, resolve_identity,
    validate_safe_owned_parent, write_owned_document,
};

const OPERATION: RepositoryOperation = RepositoryOperation::RepairItem;

/// The front matter keys a ticket's closure is held in.
const CLOSURE_KEYS: [&str; 2] = ["closed_at", "closed_by"];

/// The kinds a file's `manyhands_kind` can name.
const KINDS: [&str; 3] = ["document", "ticket", "comment"];

/// What `repair_item` is asked for.
#[derive(Clone)]
pub struct RepairItemRequest {
    /// The root of a registered repository.
    pub root: PathBuf,
    /// The kind of item the corrected source is.
    pub kind: AuthoringKind,
    /// The file to repair, relative to the repository, with forward
    /// slashes: a canonical place of an item of `kind`.
    pub path: PathBuf,
    /// The corrected complete source: a conforming item of `kind` for
    /// `path`. Its `id` is the item's identity from then on.
    pub source: String,
    /// The digest of the nonconforming file as the caller saw it on
    /// primary.
    pub expected: ExpectedPathObservation,
    pub operation_id: OperationId,
}

/// Why a repair is refused. Each is decided before anything is written.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Refusal {
    /// The path or the file is not one a repair is made of.
    NotRepairable,
    /// The corrected source is not a conforming item of the path, or
    /// changes a ticket's closure.
    InvalidSource,
    /// The corrected ID is another item's.
    AmbiguousIdentity,
}

impl Refusal {
    fn error(self, root: &Path) -> RepositoryError {
        let (kind, message) = match self {
            Self::NotRepairable => (
                RepositoryErrorKind::NotRepairable,
                "the path does not hold a file that a repair corrects",
            ),
            Self::InvalidSource => (
                RepositoryErrorKind::InvalidItemSource,
                "the corrected source is not a conforming item of the path",
            ),
            Self::AmbiguousIdentity => (
                RepositoryErrorKind::AmbiguousIdentity,
                "the corrected ID is held by another item",
            ),
        };
        RepositoryError::new(OPERATION, Some(root.to_owned()), kind, message)
    }
}

/// Whether `path` is a canonical place of an item of `kind`. A ticket
/// directory that is no ULID is no such place.
fn is_place_of(kind: AuthoringKind, path: &str) -> bool {
    matches!(
        (kind, canonical::item_path_kind(Path::new(path))),
        (AuthoringKind::Document, Ok(ItemKind::Document))
            | (AuthoringKind::Ticket, Ok(ItemKind::Ticket))
    )
}

/// The valid ID a front matter holds, if it holds one.
fn held_id(values: &serde_yaml::Mapping) -> Option<ItemId> {
    values.get("id")?.as_str()?.parse().ok()
}

/// The ID the corrected `source` gives the item at `path`, decided from
/// the input alone: nothing is read.
///
/// The path must be a canonical place of an item of `kind`, and the
/// source a conforming item of that kind for it. A ticket's source that
/// is a conforming ticket of another ID than its directory's is
/// `NotRepairable`: the path holds its ID, and a repair does not change
/// one.
pub(crate) fn corrected_id(
    kind: AuthoringKind,
    path: &str,
    source: &str,
) -> Result<ItemId, Refusal> {
    if !is_place_of(kind, path) {
        return Err(Refusal::NotRepairable);
    }
    match (kind, canonical::parse_item(Path::new(path), source)) {
        (AuthoringKind::Document, Ok(CanonicalItem::Document(document))) => Ok(document.id),
        (AuthoringKind::Ticket, Ok(CanonicalItem::Ticket(ticket))) => Ok(ticket.id),
        (AuthoringKind::Ticket, Err(_)) => {
            let elsewhere = canonical::front_matter(source)
                .as_ref()
                .and_then(held_id)
                .is_some_and(|id| {
                    let own = format!(".manyhands/tickets/{id}/ticket.md");
                    matches!(
                        canonical::parse_item(Path::new(&own), source),
                        Ok(CanonicalItem::Ticket(_))
                    )
                });
            Err(if elsewhere {
                Refusal::NotRepairable
            } else {
                Refusal::InvalidSource
            })
        }
        (
            AuthoringKind::Document | AuthoringKind::Ticket,
            Ok(CanonicalItem::Document(_) | CanonicalItem::Ticket(_) | CanonicalItem::Comment(_)),
        )
        | (AuthoringKind::Document, Err(_)) => Err(Refusal::InvalidSource),
    }
}

/// What a repair of `current`, the nonconforming file at `path`, with
/// `source`, whose ID is `id`, comes to: `Ok(true)` when the file holds
/// no valid ID and the source's is assigned, an adoption, and
/// `Ok(false)` when it holds `id` already.
///
/// - A file that is a conforming item already is not repaired.
/// - A file that names another kind than its path's is not repaired.
/// - A file that holds a valid ID keeps it.
/// - A ticket's closure fields are carried as the file's front matter
///   has them, each by its value as it is written, and are absent where
///   the file has none or its front matter cannot be read.
pub(crate) fn against_current(
    kind: AuthoringKind,
    path: &str,
    current: &[u8],
    source: &str,
    id: &ItemId,
) -> Result<bool, Refusal> {
    let text = std::str::from_utf8(current).ok();
    if text.is_some_and(|text| canonical::parse_item(Path::new(path), text).is_ok()) {
        return Err(Refusal::NotRepairable);
    }
    let values = text.and_then(canonical::front_matter);
    let names_another_kind = values
        .as_ref()
        .and_then(|values| values.get("manyhands_kind")?.as_str())
        .is_some_and(|held| held != authoring_kind_segment(&kind) && KINDS.contains(&held));
    if names_another_kind {
        return Err(Refusal::NotRepairable);
    }
    let held = values.as_ref().and_then(held_id);
    if held.as_ref().is_some_and(|held| held != id) {
        return Err(Refusal::NotRepairable);
    }
    if kind == AuthoringKind::Ticket {
        let closure = |values: Option<&serde_yaml::Mapping>| {
            CLOSURE_KEYS.map(|key| {
                values
                    .and_then(|values| values.get(key))
                    .filter(|value| !value.is_null())
                    .cloned()
            })
        };
        let corrected = canonical::front_matter(source);
        if closure(values.as_ref()) != closure(corrected.as_ref()) {
            return Err(Refusal::InvalidSource);
        }
    }
    Ok(held.is_none())
}

/// Whether `source`, the text of the file at `path`, holds `id`: as the
/// item it parses as, or, when it is no conforming item, as the valid
/// `id` of its front matter. A nonconforming file that holds a valid ID
/// keeps it when it is repaired, so the ID is that file's already.
fn holds(path: &Path, source: &str, id: &ItemId) -> bool {
    match canonical::parse_item(path, source) {
        Ok(CanonicalItem::Document(document)) => document.id == *id,
        Ok(CanonicalItem::Ticket(ticket)) => ticket.id == *id,
        Ok(CanonicalItem::Comment(comment)) => comment.id == *id,
        Err(_) => {
            canonical::front_matter(source)
                .as_ref()
                .and_then(held_id)
                .as_ref()
                == Some(id)
        }
    }
}

/// Whether `path`, in a commit's tree, is where discovery reads a file
/// from: a Markdown file under `docs/`, a ticket's or a comment's.
fn is_canonical_source(path: &str) -> bool {
    path.ends_with(".md")
        && ["docs/", ".manyhands/tickets/", ".manyhands/comments/"]
            .iter()
            .any(|prefix| path.starts_with(prefix))
}

/// Whether a file `commit` holds, other than the one at `path`, holds
/// `id`. This is asked of the commit an editing context is cut from, or
/// stands at, before the context is made: what the primary worktree
/// shows can differ from what is committed.
fn held_in_commit(
    repository: &Repository,
    commit: &git2::Commit<'_>,
    path: &Path,
    id: &ItemId,
    root: &Path,
) -> Result<bool, RepositoryError> {
    let git = |error| RepositoryError::git(OPERATION, Some(root.to_owned()), error);
    let tree = commit.tree().map_err(git)?;
    let mut held = false;
    let mut failed = None;
    let walked = tree.walk(git2::TreeWalkMode::PreOrder, |directory, entry| {
        let Some(name) = entry.name() else {
            return git2::TreeWalkResult::Ok;
        };
        match entry.kind() {
            // Only the two directories canonical files are under.
            Some(git2::ObjectType::Tree) if directory.is_empty() => {
                if name == "docs" || name == ".manyhands" {
                    git2::TreeWalkResult::Ok
                } else {
                    git2::TreeWalkResult::Skip
                }
            }
            Some(git2::ObjectType::Blob) => {
                let relative = format!("{directory}{name}");
                if Path::new(&relative) == path || !is_canonical_source(&relative) {
                    return git2::TreeWalkResult::Ok;
                }
                match repository.find_blob(entry.id()) {
                    Ok(blob) => {
                        let found = std::str::from_utf8(blob.content())
                            .is_ok_and(|source| holds(Path::new(&relative), source, id));
                        if found {
                            held = true;
                            git2::TreeWalkResult::Abort
                        } else {
                            git2::TreeWalkResult::Ok
                        }
                    }
                    Err(error) => {
                        failed = Some(error);
                        git2::TreeWalkResult::Abort
                    }
                }
            }
            _ => git2::TreeWalkResult::Ok,
        }
    });
    if let Some(error) = failed {
        return Err(git(error));
    }
    if held {
        return Ok(true);
    }
    walked.map_err(git)?;
    Ok(false)
}

/// Whether a file of the worktree `directory` other than the one at
/// `path` holds `id`; see `holds`. Such a file need not be an item the
/// index holds: two files that hold one ID are a problem there, and
/// neither is listed as an item, and a nonconforming file is none.
fn held_elsewhere(
    directory: &Path,
    path: &Path,
    id: &ItemId,
    root: &Path,
) -> Result<bool, RepositoryError> {
    let mut sources = Vec::new();
    collect_canonical_sources(directory, &mut sources, OPERATION)
        .map_err(|error| error.for_operation(OPERATION, root))?;
    Ok(sources
        .iter()
        .any(|(other, source)| other != path && holds(other, source, id)))
}

/// Whether anything but the file at `path` holds `id`: another file of
/// primary's worktree, one in the commit the item's context is cut from
/// or stands at (the tip of the item's branch, or primary's head), or
/// one in the item's context when it is there. This is the one question
/// a preview and the operation both ask, so that a preview is given
/// only of a repair the operation would not refuse for it.
pub(crate) fn held_anywhere(
    root: &Path,
    kind: AuthoringKind,
    path: &Path,
    id: &ItemId,
) -> Result<bool, RepositoryError> {
    let git = |error| RepositoryError::git(OPERATION, Some(root.to_owned()), error);
    let repository = Repository::open(root).map_err(git)?;
    let branch = format!("manyhands/{}/{id}", authoring_kind_segment(&kind));
    let base = match repository.find_branch(&branch, git2::BranchType::Local) {
        Ok(branch) => branch.get().peel_to_commit().map_err(git)?,
        Err(error) if error.code() == git2::ErrorCode::NotFound => repository
            .head()
            .and_then(|head| head.peel_to_commit())
            .map_err(git)?,
        Err(error) => return Err(git(error)),
    };
    let worktree = root.join(".manyhands/worktrees").join(id.to_string());
    Ok(held_elsewhere(root, path, id, root)?
        || held_in_commit(&repository, &base, path, id, root)?
        || (worktree.is_dir() && held_elsewhere(&worktree, path, id, root)?))
}

/// Whether some item's editing context holds, at `path`, a conforming
/// item under another ID than `id`: the file was repaired before, and
/// has its identity there. The contexts are the worktrees Git has
/// registered under an item's ID, and each file is read as it is on
/// disk, so a repair is seen whether or not the index has seen it: one
/// whose hand-off is owed, and one that was abandoned after it
/// committed.
pub(crate) fn repaired_under_another_id(
    root: &Path,
    path: &Path,
    id: &ItemId,
) -> Result<bool, RepositoryError> {
    let git = |error| RepositoryError::git(OPERATION, Some(root.to_owned()), error);
    let repository = Repository::open(root).map_err(git)?;
    let worktrees = repository.worktrees().map_err(git)?;
    Ok(worktrees
        .iter()
        .flatten()
        .filter(|name| name.parse::<ItemId>().is_ok())
        .any(|name| {
            let worktree = root.join(".manyhands/worktrees").join(name);
            // A worktree that is not there, or a path in it that cannot
            // be read as a file, holds no item.
            let file = owned_file_bytes(&worktree, path, OPERATION, root)
                .ok()
                .flatten();
            file.as_deref()
                .and_then(|bytes| std::str::from_utf8(bytes).ok())
                .and_then(|source| canonical::parse_item(path, source).ok())
                .is_some_and(|item| match item {
                    CanonicalItem::Document(document) => document.id != *id,
                    CanonicalItem::Ticket(ticket) => ticket.id != *id,
                    CanonicalItem::Comment(comment) => comment.id != *id,
                })
        }))
}

fn external_change(root: &Path, id: &ItemId, context: &Path, path: &Path) -> RepositoryError {
    RepositoryError::external_change_error(ExternalChangeDiagnostic {
        root: root.to_owned(),
        operation: OPERATION,
        item_id: id.clone(),
        context: context.to_owned(),
        path: path.to_owned(),
        expectation: ExternalChangeExpectation::Changed,
    })
}

impl RepositoryService {
    /// Replaces the nonconforming file at `request.path` with the
    /// corrected source, in the editing context of the item the source
    /// makes, and checkpoints it there with the ordinary subject for its
    /// kind. Primary is not written.
    ///
    /// The source is written as it is given, byte for byte. Its ID names
    /// the context: the branch `manyhands/<kind>/<id>` and its worktree,
    /// made from primary's head when they are not there.
    ///
    /// Refused, with nothing written and no context made: a path or a
    /// file a repair is not made of (`NotRepairable`), a source that is
    /// no conforming item of the path or changes a ticket's closure
    /// (`InvalidItemSource`), an ID another file holds on primary or in
    /// the context the ID names (`AmbiguousIdentity`), and a file on
    /// primary that is not what `request.expected` digests
    /// (`ExternalChange`).
    ///
    /// A call under an operation ID whose row an earlier call left
    /// pending goes on from what that call did: a source it wrote is
    /// found as intended and committed.
    pub fn repair_item(&self, request: RepairItemRequest) -> Result<SaveOutcome, RepositoryError> {
        let RepairItemRequest {
            root: selected,
            kind,
            path,
            source,
            expected,
            operation_id,
        } = request;
        self.require_index_available(OPERATION, Some(&selected))?;
        let (root_repository, root) = canonical_repository_root(&selected, OPERATION)?;
        let path_text = path
            .to_str()
            .ok_or_else(|| Refusal::NotRepairable.error(&root))?;
        let id = corrected_id(kind, path_text, &source).map_err(|refusal| refusal.error(&root))?;
        let target = AuthoringTarget {
            root: root.clone(),
            kind,
            item_id: id.clone(),
            intent: ContextIntent::Repair,
            operation_id,
        };
        self.reject_pending_synchronization_target(&root, &kind, &id, OPERATION)?;
        let (lease, record) = self.begin_lifecycle(
            &root_repository,
            &root,
            OPERATION,
            operation_id,
            &authoring_write_target(&target, &[path.as_path()]),
        )?;
        // Two calls of one request can run at once. One that finds the
        // row claimed by a call that is indexing under it, with no lease
        // held, leaves the row to that call: the operation's own work is
        // committed, or it would not have handed off.
        if record.is_pending && self.is_being_indexed(&root, OPERATION, operation_id)? {
            drop(lease);
            return Ok(SaveOutcome::IndexPending {
                context: item_context(&root, kind, id),
                checkpoint: LocalCheckpoint::NoChange,
            });
        }
        let effects = CallEffects::default();
        let intended = source.as_bytes();
        let result = (|| {
            // Asked for as long as no call under the operation has
            // recorded its write: a row that was begun and left with
            // nothing done is entered again as it was begun.
            if record.completed_step.is_none() {
                // The file is where its path says by the names its
                // directories list, and not only by a name a file system
                // that folds case takes for it: the source is written at
                // this path in the item's context.
                if !is_listed_file(&root, path_text)
                    .map_err(|error| RepositoryError::io(OPERATION, Some(root.clone()), error))?
                {
                    return Err(Refusal::NotRepairable.error(&root));
                }
                // What the caller saw is what primary holds, and the
                // rules are asked of that file: the context may hold the
                // corrected source already.
                let current = owned_file_bytes(&root, &path, OPERATION, &root)?
                    .filter(|bytes| ExpectedPathObservation::from_bytes(bytes) == expected)
                    .ok_or_else(|| external_change(&root, &id, &root, &path))?;
                against_current(kind, path_text, &current, &source, &id)
                    .map_err(|refusal| refusal.error(&root))?;
                // Nothing else holds the ID, and the file has not been
                // given another in some item's context. Both are asked
                // before a context is made, so that a refusal leaves
                // none.
                if held_anywhere(&root, kind, &path, &id)? {
                    return Err(Refusal::AmbiguousIdentity.error(&root));
                }
                if repaired_under_another_id(&root, &path, &id)? {
                    return Err(Refusal::NotRepairable.error(&root));
                }
            }
            let context = match self.prepare_context_unlocked(target, Some(record), &effects)? {
                ContextProvisionOutcome::Created(context)
                | ContextProvisionOutcome::Reused(context)
                | ContextProvisionOutcome::IndexPending { context } => context,
            };
            // A whole editing context is reusable by any later operation.
            effects.undone();
            self.reject_pending_synchronization_merge(&context, OPERATION)?;
            let repository = Repository::open(&context.worktree)
                .map_err(|error| RepositoryError::git(OPERATION, Some(root.clone()), error))?;
            let config = repository
                .config()
                .map_err(|error| RepositoryError::git(OPERATION, Some(root.clone()), error))?;
            let Some((identity, _)) =
                resolve_identity(&config, &config, &context.worktree, OPERATION)?
            else {
                return Ok(SaveOutcome::IdentityRequired { context });
            };
            validate_safe_owned_parent(&context.worktree, &path, OPERATION, &root)?;
            // The context's file is the nonconforming one or, where an
            // earlier call wrote and did not commit, the corrected source.
            ensure_replayed_owned_destination(
                &context.worktree,
                &path,
                intended,
                &expected,
                OPERATION,
                &context,
            )?;
            let found = observe_owned_regular_file(&context.worktree, &path, OPERATION, &root)?;
            if found != ExpectedPathObservation::from_bytes(intended) {
                effects.begin();
                ensure_safe_owned_parent(&context.worktree, &path, OPERATION, &root)?;
                self.check_failure(FailurePoint::BeforeItemWrite, OPERATION, &root)?;
                write_owned_document(&context.worktree, &path, intended, OPERATION, &root)?;
                self.advance_lifecycle(
                    &root,
                    OPERATION,
                    record,
                    LocalStep::AuthoringDestinationObserved,
                )?;
            }
            effects.begin();
            let checkpoint = self.checkpoint_owned_paths(
                &repository,
                &context,
                &identity,
                CheckpointSpec {
                    operation: OPERATION,
                    subject: &format!("Checkpoint {} {id}", authoring_kind_segment(&kind)),
                    added_paths: &[path.as_path()],
                    removed_source: None,
                },
            )?;
            self.advance_lifecycle(
                &root,
                OPERATION,
                record,
                LocalStep::AuthoringCheckpointObserved,
            )?;
            Ok(SaveOutcome::Saved {
                context,
                checkpoint,
            })
        })();
        match result {
            Ok(SaveOutcome::Saved {
                context,
                checkpoint,
            }) if !matches!(checkpoint, LocalCheckpoint::NoChange) || record.is_pending => {
                if self.handoff_post_authoritative(&root, OPERATION, record, operation_id, lease)? {
                    Ok(SaveOutcome::Saved {
                        context,
                        checkpoint: checkpoint_after_refresh(checkpoint),
                    })
                } else {
                    Ok(SaveOutcome::IndexPending {
                        context,
                        checkpoint,
                    })
                }
            }
            Ok(value) => {
                drop(lease);
                self.complete_lifecycle(&root, OPERATION, record)?;
                Ok(value)
            }
            Err(error) => {
                self.finish_authoring_lifecycle(&root, OPERATION, record, &effects, Err(error))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ID: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAV";
    const OTHER: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAW";
    const MARKER: &[u8] = b"---\nmanyhands_managed: true\n---\nText.\n";

    fn id(text: &str) -> ItemId {
        text.parse().unwrap()
    }

    fn document(id: &str) -> String {
        format!(
            "---\nmanyhands_managed: true\nmanyhands_kind: document\nid: {id}\ntitle: Guide\n---\nText.\n"
        )
    }

    fn ticket(id: &str, extra: &str) -> String {
        format!(
            "---\nmanyhands_managed: true\nmanyhands_kind: ticket\nid: {id}\ntitle: Task\ntype: task\nstatus: open\n{extra}---\nText.\n"
        )
    }

    fn ticket_path(id: &str) -> String {
        format!(".manyhands/tickets/{id}/ticket.md")
    }

    const CLOSED: &str = "closed_at: 2026-01-02T03:04:05Z\nclosed_by: Ada <ada@example.invalid>\n";

    #[test]
    fn the_corrected_id_is_the_source_s_for_a_place_of_its_kind() {
        assert_eq!(
            corrected_id(AuthoringKind::Document, "docs/a.md", &document(ID)),
            Ok(id(ID))
        );
        assert_eq!(
            corrected_id(AuthoringKind::Ticket, &ticket_path(ID), &ticket(ID, "")),
            Ok(id(ID))
        );
    }

    #[test]
    fn a_path_that_is_no_place_of_the_kind_is_not_repairable() {
        for (kind, path) in [
            (AuthoringKind::Document, "notes/a.md"),
            (AuthoringKind::Document, "docs/a.txt"),
            (AuthoringKind::Document, ".manyhands/tickets/x/ticket.md"),
            (AuthoringKind::Ticket, "docs/a.md"),
            (
                AuthoringKind::Ticket,
                ".manyhands/tickets/not-a-ulid/ticket.md",
            ),
            (AuthoringKind::Ticket, ".manyhands/comments/a/b.md"),
        ] {
            let source = match kind {
                AuthoringKind::Document => document(ID),
                AuthoringKind::Ticket => ticket(ID, ""),
            };
            assert_eq!(
                corrected_id(kind, path, &source),
                Err(Refusal::NotRepairable),
                "{path}"
            );
        }
    }

    #[test]
    fn a_source_that_is_no_item_of_the_path_is_invalid() {
        // Nonconforming itself.
        assert_eq!(
            corrected_id(
                AuthoringKind::Document,
                "docs/a.md",
                std::str::from_utf8(MARKER).unwrap()
            ),
            Err(Refusal::InvalidSource)
        );
        // Of the wrong kind for the path.
        assert_eq!(
            corrected_id(AuthoringKind::Document, "docs/a.md", &ticket(ID, "")),
            Err(Refusal::InvalidSource)
        );
        assert_eq!(
            corrected_id(AuthoringKind::Ticket, &ticket_path(ID), &document(ID)),
            Err(Refusal::InvalidSource)
        );
        // A ticket of another ID that is broken besides.
        assert_eq!(
            corrected_id(
                AuthoringKind::Ticket,
                &ticket_path(ID),
                &ticket(OTHER, "closed_at: yesterday\n")
            ),
            Err(Refusal::InvalidSource)
        );
    }

    #[test]
    fn a_conforming_ticket_of_another_id_than_its_directory_s_is_not_repairable() {
        assert_eq!(
            corrected_id(AuthoringKind::Ticket, &ticket_path(ID), &ticket(OTHER, "")),
            Err(Refusal::NotRepairable)
        );
    }

    #[test]
    fn a_file_with_no_valid_id_is_adopted_and_one_with_an_id_keeps_it() {
        let doc = AuthoringKind::Document;
        assert_eq!(
            against_current(doc, "docs/a.md", MARKER, &document(ID), &id(ID)),
            Ok(true)
        );
        // Not text at all.
        assert_eq!(
            against_current(doc, "docs/a.md", &[0xff, 0xfe], &document(ID), &id(ID)),
            Ok(true)
        );
        // A valid ID and no title.
        let untitled =
            format!("---\nmanyhands_managed: true\nmanyhands_kind: document\nid: {ID}\n---\n");
        assert_eq!(
            against_current(
                doc,
                "docs/a.md",
                untitled.as_bytes(),
                &document(ID),
                &id(ID)
            ),
            Ok(false)
        );
        assert_eq!(
            against_current(
                doc,
                "docs/a.md",
                untitled.as_bytes(),
                &document(OTHER),
                &id(OTHER)
            ),
            Err(Refusal::NotRepairable)
        );
    }

    #[test]
    fn a_conforming_file_and_one_of_another_kind_are_not_repairable() {
        let doc = AuthoringKind::Document;
        assert_eq!(
            against_current(
                doc,
                "docs/a.md",
                document(ID).as_bytes(),
                &document(ID),
                &id(ID)
            ),
            Err(Refusal::NotRepairable)
        );
        assert_eq!(
            against_current(
                doc,
                "docs/a.md",
                ticket(ID, "").as_bytes(),
                &document(ID),
                &id(ID)
            ),
            Err(Refusal::NotRepairable)
        );
        // A kind that is none at all is a broken field, and is repaired.
        let misspelt = "---\nmanyhands_managed: true\nmanyhands_kind: docment\n---\n";
        assert_eq!(
            against_current(
                doc,
                "docs/a.md",
                misspelt.as_bytes(),
                &document(ID),
                &id(ID)
            ),
            Ok(true)
        );
    }

    #[test]
    fn a_ticket_s_closure_is_carried_and_never_added_or_dropped() {
        let kind = AuthoringKind::Ticket;
        let path = ticket_path(ID);
        // Closed, with a broken field: no type.
        let broken = format!(
            "---\nmanyhands_managed: true\nmanyhands_kind: ticket\nid: {ID}\ntitle: Task\nstatus: done\n{CLOSED}---\n"
        );
        let check = |current: &str, source: &str| {
            against_current(kind, &path, current.as_bytes(), source, &id(ID))
        };
        assert_eq!(check(&broken, &ticket(ID, CLOSED)), Ok(false));
        // Dropped, changed, and half of it.
        assert_eq!(check(&broken, &ticket(ID, "")), Err(Refusal::InvalidSource));
        assert_eq!(
            check(
                &broken,
                &ticket(
                    ID,
                    "closed_at: 2026-01-02T03:04:06Z\nclosed_by: Ada <ada@example.invalid>\n"
                )
            ),
            Err(Refusal::InvalidSource)
        );
        // Added to an open ticket.
        let open = format!(
            "---\nmanyhands_managed: true\nmanyhands_kind: ticket\nid: {ID}\ntitle: Task\nstatus: open\n---\n"
        );
        assert_eq!(check(&open, &ticket(ID, "")), Ok(false));
        assert_eq!(
            check(&open, &ticket(ID, CLOSED)),
            Err(Refusal::InvalidSource)
        );
        // Front matter that cannot be read at all.
        let unreadable = "---\n: : :\n\t- [\n---\n";
        assert_eq!(check(unreadable, &ticket(ID, "")), Ok(true));
        assert_eq!(
            check(unreadable, &ticket(ID, CLOSED)),
            Err(Refusal::InvalidSource)
        );
    }
}
