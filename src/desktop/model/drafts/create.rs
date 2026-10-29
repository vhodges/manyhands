//! A new item's create, as pure decisions (design, Create (decision 3);
//! R-20b, R-21, R-42): the command a new item's draft is created with, the
//! base a create leaves, and the base a new item's draft goes on over once
//! its item turns out to exist.
//!
//! The registry sends and settles the create (`services::drafts::create`).
//! Nothing here holds text but the intents and bases it gives, which derive
//! no `Debug`.

use std::path::PathBuf;

use manyhands::{
    drafts::{DraftBase, DraftFields, DraftTarget, ItemKind, is_new_item_base},
    repository::ItemDto,
    results::ResultCode,
};

use super::save::base_after_save;
use crate::desktop::services::requests::Intent;

/// The command that creates the new item `target` with `fields`, at the
/// path `base` holds: `DocumentCreate`, or `TicketCreate` with the draft's
/// fields, dependencies and parent, and no initials, which are the
/// repository's settings (R-21). `None` unless `base` is a new item's
/// (R-20), for a comment's draft, and for fields of another kind than the
/// target's.
pub(crate) fn create_intent(
    target: &DraftTarget,
    fields: &DraftFields,
    base: &DraftBase,
) -> Option<Intent> {
    if target.comment.is_some() || !is_new_item_base(base) {
        return None;
    }
    let root = PathBuf::from(&target.repository);
    let id = target.item_id.clone();
    match (target.item_kind, fields) {
        (ItemKind::Document, DraftFields::Document { title, body }) => {
            Some(Intent::DocumentCreate {
                root,
                id,
                path: PathBuf::from(&base.path),
                title: title.clone(),
                body: body.clone(),
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
        ) => Some(Intent::TicketCreate {
            root,
            id,
            title: title.clone(),
            ticket_type: ticket_type.clone(),
            status: status.clone(),
            project: project.clone(),
            team: team.clone(),
            body: body.clone(),
            deps: deps.clone(),
            parent: parent.clone(),
        }),
        _ => None,
    }
}

/// The base a create of `submitted` leaves (R-20b), by D2's unverified
/// rule (`base_after_save`): a read with exactly the submitted fields is
/// the new base; otherwise, another actor having changed the item between
/// the create and the read, or the read having failed, the submitted
/// fields with no observation and the source `""`. Either way it is no new
/// item's base: the marker clears, and another actor's change shows in the
/// review, with theirs as the default.
pub(crate) fn base_after_create(
    submitted: &DraftFields,
    read: Result<&ItemDto, ResultCode>,
    previous: &DraftBase,
) -> DraftBase {
    let base = base_after_save(submitted, read, previous);
    match base.observation {
        Some(_) => base,
        None => unmarked(base),
    }
}

/// A new item's base whose item exists after all (R-42; a create's
/// `occupied_path` for its own item): an ordinary unverified base over the
/// fields it holds, with no observation and the source `""`, which the
/// review compares with the item.
pub(crate) fn unmarked(base: DraftBase) -> DraftBase {
    DraftBase {
        observation: None,
        source: Some(String::new()),
        ..base
    }
}
