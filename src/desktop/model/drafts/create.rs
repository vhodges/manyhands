//! A new item's create, as pure decisions (design, Create (decision 3);
//! R-20, R-20b, R-21, R-42): the draft a create dialog begins, the item its
//! tab shows until it is created, the command a new item's draft is created
//! with, the base a create leaves, and the base a new item's draft goes on
//! over once its item turns out to exist.
//!
//! The registry sends and settles the create (`services::drafts::create`).
//! Nothing here holds text but the fields, intents, bases and item it
//! gives, which derive no `Debug`, the library's item aside.

use std::path::PathBuf;

use manyhands::{
    canonical::ItemId,
    drafts::{
        BaseContext, ContextKind, DraftBase, DraftFields, DraftTarget, ItemKind, is_new_item_base,
    },
    repository::{
        ClosureDto, ClosureState, IndexState, IndexStateDto, ItemContextDto, ItemContextKind,
        ItemDto, ItemDtoKind,
    },
    results::ResultCode,
};

use super::save::base_after_save;
use crate::desktop::services::requests::Intent;

/// A new ticket's type until its dialog's Type says otherwise (decision 3).
pub(crate) const DEFAULT_TYPE: &str = "task";
/// A new ticket's status (decision 3; R-21).
pub(crate) const DEFAULT_STATUS: &str = "open";

/// A new ticket's fields: `title` and `ticket_type`, the status `open`, no
/// project, team, dependency or parent, and an empty body.
pub(crate) fn ticket_fields(title: &str, ticket_type: &str) -> DraftFields {
    DraftFields::Ticket {
        title: title.to_owned(),
        ticket_type: ticket_type.to_owned(),
        status: DEFAULT_STATUS.to_owned(),
        project: None,
        team: None,
        deps: Vec::new(),
        parent: None,
        body: String::new(),
    }
}

/// A new document's fields: `title`, and an empty body.
pub(crate) fn document_fields(title: &str) -> DraftFields {
    DraftFields::Document {
        title: title.to_owned(),
        body: String::new(),
    }
}

/// Where the new ticket `id` is written.
pub(crate) fn ticket_path(id: &ItemId) -> String {
    format!(".manyhands/tickets/{id}/ticket.md")
}

/// The draft of the new item `id`, of `kind`, in the repository `root`, at
/// `path` with `fields` (R-20): its target, and its base, which has neither
/// observation nor source and holds the path and the fields, in the
/// repository's primary context. `branch` is the primary branch as the
/// repository list gives it; nothing here names one.
pub(crate) fn new_item_draft(
    root: &str,
    branch: Option<String>,
    kind: ItemKind,
    id: ItemId,
    path: String,
    fields: &DraftFields,
) -> (DraftTarget, DraftBase) {
    let target = DraftTarget {
        repository: root.to_owned(),
        item_kind: kind,
        item_id: id,
        comment: None,
    };
    let base = DraftBase {
        observation: None,
        path,
        context: BaseContext {
            kind: ContextKind::Primary,
            branch,
            worktree: root.to_owned(),
            head_oid: None,
        },
        source: None,
        fields: fields.clone(),
    };
    (target, base)
}

/// What the tab of the new item `id`, of `kind`, shows of it until its
/// tab reads it (design, Create: the new tab opens editing, with its header
/// built from the draft's fields): the draft's named fields over its base's
/// path and context kind, open, with no body, short code, relationship,
/// readiness, other key or problem. A field left empty is not shown. Once
/// `created`, its context kind is Active, whatever the base holds: every
/// create commits in the item's own private context, the one its Create
/// prepare made ready (Part D fix 2, N2).
pub(crate) fn shown_new_item(
    kind: ItemDtoKind,
    id: &ItemId,
    base: &DraftBase,
    fields: &DraftFields,
    created: bool,
) -> ItemDto {
    let named = |value: &str| (!value.is_empty()).then(|| value.to_owned());
    let (title, ticket_type, status, project, team) = match fields {
        DraftFields::Ticket {
            title,
            ticket_type,
            status,
            project,
            team,
            ..
        } => (
            named(title),
            named(ticket_type),
            named(status),
            project.clone(),
            team.clone(),
        ),
        DraftFields::Document { title, .. } => (named(title), None, None, None, None),
        DraftFields::Comment { .. } => (None, None, None, None, None),
    };
    let context = &base.context;
    ItemDto {
        id: Some(id.to_string()),
        kind,
        path: base.path.clone(),
        title,
        ticket_type,
        status,
        project,
        team,
        closure: (kind == ItemDtoKind::Ticket).then_some(ClosureDto {
            state: ClosureState::Open,
            closed_at: None,
            closed_by: None,
        }),
        slug: None,
        parent: None,
        deps: Vec::new(),
        readiness: None,
        unknown_metadata: serde_json::Map::new(),
        body: None,
        source: None,
        observation: None,
        context: ItemContextDto {
            kind: match context.kind {
                _ if created => ItemContextKind::Active,
                ContextKind::Primary => ItemContextKind::Primary,
                ContextKind::Unverified => ItemContextKind::Unverified,
                ContextKind::Active => ItemContextKind::Active,
            },
            branch: context.branch.clone(),
            worktree: context.worktree.clone(),
            head_oid: context.head_oid.clone(),
        },
        changed_at: None,
        change_source: None,
        problems: Vec::new(),
        index: IndexStateDto {
            state: IndexState::Current,
            refreshed_at: None,
        },
    }
}

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
