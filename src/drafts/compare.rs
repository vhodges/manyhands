//! Pure comparison of a draft with its item, the item's target state, and
//! whether the draft may be restored. Nothing here reads or writes a file.
//!
//! `TargetState` holds the item's fields, so it derives no `Debug`.
//! `Field`, `Change`, `Comparison`, `Restore` and `RefusalReason` hold no
//! text and do.

use super::schema::{BaseContext, ContextKind, Draft, DraftBase, DraftFields};
use super::store::Held;
use crate::repository::{
    ClosureState, CommentListDto, ItemContextDto, ItemContextKind, ItemDto, ItemDtoKind,
};
use crate::results::ResultCode;

impl DraftFields {
    /// The editable fields of a conforming item; `None` for a
    /// nonconforming entry. A ticket's `deps` are its dependencies' IDs in
    /// file order and its `parent` its parent's ID. A dependency the
    /// library ignored is not among them.
    pub fn of(item: &ItemDto) -> Option<DraftFields> {
        item.id.as_ref()?;
        let title = item.title.clone()?;
        let body = item.body.clone()?;
        Some(match item.kind {
            ItemDtoKind::Document => DraftFields::Document { title, body },
            ItemDtoKind::Ticket => DraftFields::Ticket {
                title,
                ticket_type: item.ticket_type.clone()?,
                status: item.status.clone()?,
                project: item.project.clone(),
                team: item.team.clone(),
                deps: item.deps.iter().map(|dep| dep.id.clone()).collect(),
                parent: item.parent.as_ref().map(|parent| parent.id.clone()),
                body,
            },
        })
    }
}

impl DraftBase {
    /// The item as read, as a draft's base: its observation, path,
    /// context, source and fields. `None` for a nonconforming entry.
    pub fn of(item: &ItemDto) -> Option<DraftBase> {
        Some(DraftBase {
            observation: item.observation.clone(),
            path: item.path.clone(),
            context: BaseContext {
                kind: context_kind(item.context.kind),
                branch: item.context.branch.clone(),
                worktree: item.context.worktree.clone(),
                head_oid: item.context.head_oid.clone(),
            },
            source: item.source.clone(),
            fields: DraftFields::of(item)?,
        })
    }
}

fn context_kind(kind: ItemContextKind) -> ContextKind {
    match kind {
        ItemContextKind::Primary => ContextKind::Primary,
        ItemContextKind::Unverified => ContextKind::Unverified,
        ItemContextKind::Active => ContextKind::Active,
    }
}

/// An editable field, in the order a comparison lists them.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Field {
    Title,
    Type,
    Status,
    Project,
    Team,
    Deps,
    Parent,
    Body,
}

const FIELDS: [Field; 8] = [
    Field::Title,
    Field::Type,
    Field::Status,
    Field::Project,
    Field::Team,
    Field::Deps,
    Field::Parent,
    Field::Body,
];

/// How one field differs from the base.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Change {
    /// Neither side changed it.
    None,
    /// Only the draft changed it.
    Mine,
    /// Only the current item changed it.
    Theirs,
    /// Both changed it, to the same value.
    BothSame,
    /// Both changed it, differently.
    Both,
}

/// A three-way comparison, field by field, in a fixed order with the body
/// last. It holds no text.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Comparison {
    pub fields: Vec<(Field, Change)>,
}

/// One field's value, borrowed, for comparison only.
#[derive(PartialEq, Eq)]
enum Value<'a> {
    Text(&'a str),
    Optional(Option<&'a str>),
    List(&'a [String]),
}

fn value(fields: &DraftFields, field: Field) -> Option<Value<'_>> {
    match (fields, field) {
        (DraftFields::Document { title, .. }, Field::Title)
        | (DraftFields::Ticket { title, .. }, Field::Title) => Some(Value::Text(title)),
        (DraftFields::Document { body, .. }, Field::Body)
        | (DraftFields::Ticket { body, .. }, Field::Body)
        | (DraftFields::Comment { body }, Field::Body) => Some(Value::Text(body)),
        (DraftFields::Ticket { ticket_type, .. }, Field::Type) => Some(Value::Text(ticket_type)),
        (DraftFields::Ticket { status, .. }, Field::Status) => Some(Value::Text(status)),
        (DraftFields::Ticket { project, .. }, Field::Project) => {
            Some(Value::Optional(project.as_deref()))
        }
        (DraftFields::Ticket { team, .. }, Field::Team) => Some(Value::Optional(team.as_deref())),
        (DraftFields::Ticket { deps, .. }, Field::Deps) => Some(Value::List(deps)),
        (DraftFields::Ticket { parent, .. }, Field::Parent) => {
            Some(Value::Optional(parent.as_deref()))
        }
        _ => None,
    }
}

/// Compares the draft (`mine`) and the current item (`theirs`) with the
/// base, field by field. A field is listed when any of the three has it.
/// There is no text merge.
pub fn compare(base: &DraftFields, theirs: &DraftFields, mine: &DraftFields) -> Comparison {
    let fields = FIELDS
        .into_iter()
        .filter_map(|field| {
            let (base, theirs, mine) =
                (value(base, field), value(theirs, field), value(mine, field));
            if base.is_none() && theirs.is_none() && mine.is_none() {
                return None;
            }
            let change = match (mine != base, theirs != base) {
                (false, false) => Change::None,
                (true, false) => Change::Mine,
                (false, true) => Change::Theirs,
                (true, true) if mine == theirs => Change::BothSame,
                (true, true) => Change::Both,
            };
            Some((field, change))
        })
        .collect();
    Comparison { fields }
}

/// Where a draft's item stands now, against the draft's base.
#[derive(Clone, PartialEq, Eq)]
pub enum TargetState {
    /// The base observation equals the read's.
    Current,
    Changed {
        theirs: DraftFields,
        comparison: Comparison,
        /// The read's context kind or branch differs from the base's.
        context_changed: bool,
    },
    /// The item's fields equal the draft's.
    AlreadySaved,
    /// The base has no observation to check: compared with the fields it
    /// holds.
    Unverified {
        theirs: DraftFields,
        comparison: Comparison,
    },
    TicketClosed,
    ItemNotFound,
    Nonconforming,
    /// Not registered, inaccessible, not a repository.
    RepositoryUnavailable(ResultCode),
    /// Busy, index unavailable, anything else: try again.
    ReadFailed(ResultCode),
    /// Comment drafts only: the reply's parent is not in the thread.
    ParentNotFound,
}

/// The state a failed read gives.
fn read_error(code: ResultCode) -> TargetState {
    match code {
        ResultCode::RepositoryNotRegistered
        | ResultCode::RepositoryInaccessible
        | ResultCode::NotRepository
        | ResultCode::NotRepositoryRoot
        | ResultCode::RepositoryNotEnabled => TargetState::RepositoryUnavailable(code),
        ResultCode::ItemNotFound => TargetState::ItemNotFound,
        code => TargetState::ReadFailed(code),
    }
}

fn lifecycle_closed(item: &ItemDto) -> bool {
    item.closure
        .as_ref()
        .is_some_and(|closure| closure.state == ClosureState::Closed)
}

fn context_changed(base: &BaseContext, read: &ItemContextDto) -> bool {
    base.kind != context_kind(read.kind) || base.branch != read.branch
}

/// Classifies an item draft against a read of its item. In order: a read
/// error by code; a lifecycle-closed ticket, whatever else holds; a
/// nonconforming read; fields equal to the draft's; an unverified base;
/// an equal observation; otherwise changed.
///
/// A draft with no base at all is unverified, compared with the item's
/// own fields as the base, so only the draft's changes show. A comment
/// draft is classified by its item alone; `classify_comment` also checks
/// its thread.
pub fn classify(draft: &Draft, item: Result<&ItemDto, ResultCode>) -> TargetState {
    let item = match item {
        Ok(item) => item,
        Err(code) => return read_error(code),
    };
    if lifecycle_closed(item) {
        return TargetState::TicketClosed;
    }
    if draft.target.comment.is_some() {
        return TargetState::Current;
    }
    let Some(theirs) = DraftFields::of(item) else {
        return TargetState::Nonconforming;
    };
    if theirs == draft.current {
        return TargetState::AlreadySaved;
    }
    let base = match &draft.base {
        Some(base) if base.observation.is_some() => base,
        Some(base) => {
            let comparison = compare(&base.fields, &theirs, &draft.current);
            return TargetState::Unverified { theirs, comparison };
        }
        None => {
            let comparison = compare(&theirs, &theirs, &draft.current);
            return TargetState::Unverified { theirs, comparison };
        }
    };
    if base.observation == item.observation {
        return TargetState::Current;
    }
    let comparison = compare(&base.fields, &theirs, &draft.current);
    TargetState::Changed {
        theirs,
        comparison,
        context_changed: context_changed(&base.context, &item.context),
    }
}

/// Classifies a comment draft by its item and its thread. A comment is
/// never edited, so the only staleness is whether its target can still
/// take it: the item gone or closed, the repository unavailable, the read
/// failed, or a reply's parent not in the thread. A list that is not
/// complete is a failed read, never a missing parent.
pub fn classify_comment(
    draft: &Draft,
    item: Result<&ItemDto, ResultCode>,
    comments: Result<&CommentListDto, ResultCode>,
) -> TargetState {
    match item {
        Err(code) => return read_error(code),
        Ok(item) if lifecycle_closed(item) => return TargetState::TicketClosed,
        Ok(_) => {}
    }
    let comments = match comments {
        Ok(comments) => comments,
        Err(code) => return read_error(code),
    };
    if !comments.complete {
        // The last refresh could not read the whole comment directory.
        return TargetState::ReadFailed(ResultCode::IndexUnavailable);
    }
    let parent = draft
        .target
        .comment
        .as_ref()
        .and_then(|comment| comment.parent_id.as_ref())
        .map(ToString::to_string);
    match parent {
        Some(parent)
            if !comments
                .items
                .iter()
                .any(|comment| comment.id.as_deref() == Some(&*parent)) =>
        {
            TargetState::ParentNotFound
        }
        _ => TargetState::Current,
    }
}

/// Why a draft may not be restored into an editor.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RefusalReason {
    /// Another process holds the draft's claim.
    OpenElsewhere,
    /// This store holds the claim: the draft is already restored here, and
    /// "Show" focuses its tab. Restoring is not needed.
    HeldHere,
    /// The comment composer arrives in a later release.
    CommentDraft,
    TicketClosed,
    ItemNotFound,
    Nonconforming,
    ParentNotFound,
    /// Possible again once the repository is registered at the same root.
    RepositoryUnavailable,
    /// Refused for now; Retry is offered.
    ReadFailed,
}

/// Whether a draft may be restored.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Restore {
    Allowed,
    /// Allowed, with the comparison shown; a save needs a review first.
    AllowedFlagged,
    Refused(RefusalReason),
}

/// The restore policy: who holds the draft first, then whether it is a
/// comment draft, then its target state.
pub fn restore_policy(draft: &Draft, state: &TargetState, held: Held) -> Restore {
    match held {
        Held::Elsewhere => return Restore::Refused(RefusalReason::OpenElsewhere),
        Held::Here => return Restore::Refused(RefusalReason::HeldHere),
        Held::No => {}
    }
    if draft.target.comment.is_some() {
        return Restore::Refused(RefusalReason::CommentDraft);
    }
    match state {
        TargetState::Current | TargetState::AlreadySaved => Restore::Allowed,
        TargetState::Changed { .. } | TargetState::Unverified { .. } => Restore::AllowedFlagged,
        TargetState::TicketClosed => Restore::Refused(RefusalReason::TicketClosed),
        TargetState::ItemNotFound => Restore::Refused(RefusalReason::ItemNotFound),
        TargetState::Nonconforming => Restore::Refused(RefusalReason::Nonconforming),
        TargetState::ParentNotFound => Restore::Refused(RefusalReason::ParentNotFound),
        TargetState::RepositoryUnavailable(_) => {
            Restore::Refused(RefusalReason::RepositoryUnavailable)
        }
        TargetState::ReadFailed(_) => Restore::Refused(RefusalReason::ReadFailed),
    }
}
