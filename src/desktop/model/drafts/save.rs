//! A draft's save, as pure decisions: the command a revision is saved
//! with, what the save's answer does to the draft, and the base the save
//! leaves.
//!
//! A save retires only the revision it saved. That rule is the registry's
//! (`services::drafts::save`); this module decides only what an answer
//! means and what the new base is.

use std::path::PathBuf;

use manyhands::{
    drafts::{DraftBase, DraftFields, DraftTarget, ItemKind},
    repository::{ItemDto, RelationshipWrite},
    results::{CheckpointEffect, Outcome, ResultCode},
};

use crate::desktop::services::requests::Intent;

/// What a finished save does to its draft.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum SaveVerdict {
    /// The revision is in the repository: it may retire.
    Retire,
    /// The file was written and not committed; F2 holds a pending
    /// operation, and a Retry of the same request finishes it.
    Pending,
    /// Nothing the draft holds is in the repository: it stays as it is.
    Keep,
}

/// `Retire` exactly for a committed checkpoint, or an unchanged one that
/// succeeded or found nothing to do; `Pending` for a pending checkpoint;
/// `Keep` otherwise.
pub(crate) fn save_verdict(outcome: Outcome, checkpoint: CheckpointEffect) -> SaveVerdict {
    match (checkpoint, outcome) {
        (CheckpointEffect::Committed, _)
        | (CheckpointEffect::Unchanged, Outcome::Success | Outcome::Noop) => SaveVerdict::Retire,
        (CheckpointEffect::Pending, _) => SaveVerdict::Pending,
        _ => SaveVerdict::Keep,
    }
}

/// The base a save leaves (ruling R5). A save returns no observation, so
/// the item is read again: a read with exactly the submitted fields is the
/// new base. Otherwise, someone else's change between the commit and the
/// read or a read that failed, the base is the submitted fields with no
/// observation, unverified, where the previous base was.
pub(crate) fn base_after_save(
    submitted: &DraftFields,
    read: Result<&ItemDto, ResultCode>,
    previous: &DraftBase,
) -> DraftBase {
    if let Ok(item) = read
        && let Some(base) = DraftBase::of(item)
        && base.fields == *submitted
    {
        return base;
    }
    DraftBase {
        observation: None,
        fields: submitted.clone(),
        ..previous.clone()
    }
}

/// A ticket's `deps` write (ruling R13): `Unchanged` when equal to the
/// base's, so an entry the library ignored stays in the file; `Clear`
/// when emptied; `Set` otherwise.
pub(crate) fn deps_write(mine: &[String], base: &[String]) -> RelationshipWrite<Vec<String>> {
    if mine == base {
        RelationshipWrite::Unchanged
    } else if mine.is_empty() {
        RelationshipWrite::Clear
    } else {
        RelationshipWrite::Set(mine.to_vec())
    }
}

/// A ticket's `parent` write, by the rule of `deps_write`.
pub(crate) fn parent_write(
    mine: Option<&String>,
    base: Option<&String>,
) -> RelationshipWrite<String> {
    match mine {
        _ if mine == base => RelationshipWrite::Unchanged,
        None => RelationshipWrite::Clear,
        Some(parent) => RelationshipWrite::Set(parent.clone()),
    }
}

/// The command that saves `fields` over `base`, for `target`. `None` when
/// the base is unverified (it has no observation to send), for a
/// comment's draft, and for fields of another kind than the target's.
pub(crate) fn save_intent(
    target: &DraftTarget,
    fields: &DraftFields,
    base: &DraftBase,
) -> Option<Intent> {
    if target.comment.is_some() {
        return None;
    }
    let observation = base.observation.clone()?;
    let root = PathBuf::from(&target.repository);
    let id = target.item_id.clone();
    match (target.item_kind, fields, &base.fields) {
        (ItemKind::Document, DraftFields::Document { title, body }, _) => {
            Some(Intent::DocumentSave {
                root,
                id,
                title: title.clone(),
                body: body.clone(),
                observation: Some(observation),
            })
        }
        (
            ItemKind::Ticket,
            DraftFields::Ticket {
                title,
                ticket_type,
                status,
                project,
                team,
                deps,
                parent,
                body,
            },
            DraftFields::Ticket {
                deps: base_deps,
                parent: base_parent,
                ..
            },
        ) => Some(Intent::TicketSave {
            root,
            id,
            title: title.clone(),
            ticket_type: ticket_type.clone(),
            status: status.clone(),
            project: project.clone(),
            team: team.clone(),
            body: body.clone(),
            deps: deps_write(deps, base_deps),
            parent: parent_write(parent.as_ref(), base_parent.as_ref()),
            observation,
        }),
        _ => None,
    }
}

#[cfg(test)]
#[path = "save_tests.rs"]
mod tests;
