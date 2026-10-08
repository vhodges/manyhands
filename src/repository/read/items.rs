//! Document and ticket lists, and complete reads of one item.
//!
//! A list is built from the index alone and opens no item file. A complete
//! read finds where the item is in the index and then reads that one file,
//! through a guarded reader that follows no symbolic link.

use std::{
    cmp::Reverse,
    collections::{BTreeMap, BTreeSet},
    fs, io,
    path::{Component, Path, PathBuf},
    time::UNIX_EPOCH,
};

use rusqlite::{Connection, OptionalExtension};
use time::OffsetDateTime;

use super::{
    ChangeSource, ClosureDto, ClosureState, DependencyDto, DependencyState, IndexState,
    IndexStateDto, ItemContextDto, ItemContextKind, ItemDto, ItemDtoKind, ItemListDto, ProblemDto,
    REFRESH_INDEX_ACTION, ReadError, ReadinessState, ResolvedRepository,
    graph::{TicketGraph, TicketNode},
    index_state, root_action,
};
use crate::{
    canonical::{self, ItemId},
    repository::{GuardedFile, RepositoryOperation, RepositoryService, discovery::UnknownMetadata},
    results::{ProblemCode, RecoveryAction, ResultCode, absolute_path_string, timestamp_string},
};

/// Which tickets a list returns, by lifecycle closure metadata: a ticket is
/// closed when it carries `closed_at`, whatever its `status` text says.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ClosureFilter {
    Open,
    Closed,
    #[default]
    All,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReadinessFilter {
    Ready,
    Blocked,
}

/// Which tickets `list_tickets` returns. `status`, `ticket_type` and
/// `project` each match the whole stored value, case-sensitively; an unset
/// filter matches every ticket. `slug` matches a ticket's whole short code
/// without regard to ASCII case, and never a ticket that has none.
/// `readiness` keeps the open tickets that are ready, or the ones that are
/// blocked, and never a closed ticket. The default matches all of them.
///
/// A filter chooses which tickets are returned and never what a ticket
/// depends on: readiness is decided against every ticket the index holds.
///
/// A nonconforming entry has no metadata to match and is never removed.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TicketFilter {
    pub status: Option<String>,
    pub ticket_type: Option<String>,
    pub project: Option<String>,
    pub closure: ClosureFilter,
    pub slug: Option<String>,
    pub readiness: Option<ReadinessFilter>,
}

impl TicketFilter {
    /// `graph` is of every ticket, whatever the filter leaves out.
    pub(super) fn matches(&self, item: &StoredItem, graph: &TicketGraph) -> bool {
        let equals = |wanted: &Option<String>, stored: &Option<String>| {
            wanted
                .as_ref()
                .is_none_or(|wanted| stored.as_ref() == Some(wanted))
        };
        equals(&self.status, &item.status)
            && equals(&self.ticket_type, &item.ticket_type)
            && equals(&self.project, &item.project)
            && match self.closure {
                ClosureFilter::Open => item.closed_at.is_none(),
                ClosureFilter::Closed => item.closed_at.is_some(),
                ClosureFilter::All => true,
            }
            && self.slug.as_ref().is_none_or(|wanted| {
                item.relationships
                    .slug
                    .as_ref()
                    .is_some_and(|slug| slug.eq_ignore_ascii_case(wanted))
            })
            && self.readiness.is_none_or(|wanted| {
                graph.readiness_state(&item.id)
                    == Some(match wanted {
                        ReadinessFilter::Ready => ReadinessState::Ready,
                        ReadinessFilter::Blocked => ReadinessState::Blocked,
                    })
            })
    }
}

/// The stored problems that say a file is not an item. Only these make a
/// nonconforming entry; a problem about a branch, a context, a comment or
/// an unreadable file is the index status read's to report.
const CONFORMITY_PROBLEMS: [ProblemCode; 7] = [
    ProblemCode::MissingFrontMatter,
    ProblemCode::MalformedFrontMatter,
    ProblemCode::MissingField,
    ProblemCode::InvalidField,
    ProblemCode::KindPathMismatch,
    ProblemCode::InvalidPath,
    ProblemCode::DuplicateId,
];

/// The problems an ignored relationship value is stored as, per item.
const RELATIONSHIP_PROBLEMS: [ProblemCode; 5] = [
    ProblemCode::RelationshipWrongType,
    ProblemCode::RelationshipInvalidId,
    ProblemCode::RelationshipSelfReference,
    ProblemCode::DuplicateDependency,
    ProblemCode::InvalidSlug,
];

/// Where item worktrees are, below the repository root.
const WORKTREES_DIRECTORY: &str = ".manyhands/worktrees";

const DOCUMENT_PREFIX: &str = "docs/";
const TICKET_PREFIX: &str = ".manyhands/tickets/";

impl ItemDtoKind {
    /// The index stores a kind under its contract name.
    fn from_stored(stored: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.as_str() == stored)
    }

    /// Where a file of this kind lives, as a path prefix.
    fn path_prefix(self) -> &'static str {
        match self {
            Self::Document => DOCUMENT_PREFIX,
            Self::Ticket => TICKET_PREFIX,
        }
    }
}

/// A context row: one working tree the index observed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct StoredContext {
    kind: ItemContextKind,
    branch: Option<String>,
    pub(super) worktree: String,
    head_oid: Option<String>,
}

impl StoredContext {
    pub(super) fn dto(&self) -> ItemContextDto {
        ItemContextDto {
            kind: self.kind,
            branch: self.branch.clone(),
            worktree: self.worktree.clone(),
            head_oid: self.head_oid.clone(),
        }
    }
}

/// An item row with its context.
pub(super) struct StoredItem {
    /// The row's own key in the index, which its comments are stored under.
    pub(super) row_id: i64,
    pub(super) context: StoredContext,
    pub(super) id: String,
    pub(super) kind: ItemDtoKind,
    pub(super) path: String,
    pub(super) title: String,
    ticket_type: Option<String>,
    status: Option<String>,
    project: Option<String>,
    team: Option<String>,
    pub(super) closed_at: Option<i64>,
    closed_by: Option<String>,
    unknown: UnknownMetadata,
    pub(super) relationships: Relationships,
    activity_at: i64,
    change_source: ChangeSource,
}

/// A ticket's short code, parent and dependencies as its file gives them,
/// and the problems of the values that were ignored. Empty for a document.
///
/// `deps` is in the file's order. Nothing here says what a target is: that
/// is decided against the items the index holds, each time it is read.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct Relationships {
    pub(super) slug: Option<String>,
    parent: Option<String>,
    deps: Vec<String>,
    /// Each with the item ID it is about, when it is about one.
    problems: Vec<(ProblemCode, Option<String>)>,
}

impl From<&canonical::TicketRelationships> for Relationships {
    fn from(view: &canonical::TicketRelationships) -> Self {
        Self {
            slug: view.slug.clone(),
            parent: view.parent.as_ref().map(ToString::to_string),
            deps: view.deps.iter().map(ToString::to_string).collect(),
            problems: view
                .problems
                .iter()
                .map(|problem| {
                    (
                        ProblemCode::from(problem.code),
                        problem.detail.as_ref().map(ToString::to_string),
                    )
                })
                .collect(),
        }
    }
}

/// What an ID names among the items and comments the index holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Target {
    Document,
    Comment,
    OpenTicket,
    ClosedTicket,
}

/// Everything the index holds that has an ID, by that ID, as the target of
/// a relationship. An ID that is not here is in no context the index has
/// seen, and a later fetch or merge may still bring a ticket for it.
pub(super) struct Targets<'a>(BTreeMap<&'a str, Target>);

impl<'a> Targets<'a> {
    /// `rows` holds one row for each item, its effective copy, and
    /// `comments` the ID of every stored comment.
    pub(super) fn of(rows: &[&'a StoredItem], comments: &'a [String]) -> Self {
        let mut targets: BTreeMap<&str, Target> = comments
            .iter()
            .map(|comment| (comment.as_str(), Target::Comment))
            .collect();
        // An item's row decides what its ID names.
        targets.extend(rows.iter().map(|row| {
            let target = match (row.kind, row.closed_at) {
                (ItemDtoKind::Document, _) => Target::Document,
                (ItemDtoKind::Ticket, None) => Target::OpenTicket,
                (ItemDtoKind::Ticket, Some(_)) => Target::ClosedTicket,
            };
            (row.id.as_str(), target)
        }));
        Self(targets)
    }

    /// What a `deps` entry or a `parent` names: a ticket in some state, or
    /// `None` for something that is not a ticket.
    fn state(&self, id: &str) -> Option<DependencyState> {
        match self.0.get(id) {
            Some(Target::Document | Target::Comment) => None,
            Some(Target::OpenTicket) => Some(DependencyState::Open),
            Some(Target::ClosedTicket) => Some(DependencyState::Closed),
            None => Some(DependencyState::Unresolved),
        }
    }

    /// A ticket as the graph takes it: without a `parent` or a `deps` entry
    /// that names a document or a comment, which names no ticket.
    fn node(&self, id: &str, closed: bool, relationships: &Relationships) -> TicketNode {
        let names_a_ticket = |id: &&String| self.state(id).is_some();
        TicketNode {
            id: id.to_owned(),
            closed,
            parent: relationships
                .parent
                .as_ref()
                .filter(names_a_ticket)
                .cloned(),
            deps: relationships
                .deps
                .iter()
                .filter(names_a_ticket)
                .cloned()
                .collect(),
        }
    }

    /// The ticket in `row` as the graph takes it.
    fn stored_node(&self, row: &StoredItem) -> TicketNode {
        self.node(&row.id, row.closed_at.is_some(), &row.relationships)
    }
}

/// What a ticket's relationships are read against: what every ID names,
/// and the graph of every ticket, both from the effective copy of each
/// item the index holds.
pub(super) struct Related<'a> {
    targets: Targets<'a>,
    /// The effective copy of every ticket, which the graph was made from.
    tickets: Vec<&'a StoredItem>,
    pub(super) graph: TicketGraph,
}

impl<'a> Related<'a> {
    /// `rows` holds one row for each item, its effective copy, and
    /// `comments` the ID of every stored comment.
    pub(super) fn of(rows: &[&'a StoredItem], comments: &'a [String]) -> Self {
        let targets = Targets::of(rows, comments);
        let tickets: Vec<&StoredItem> = rows
            .iter()
            .copied()
            .filter(|row| row.kind == ItemDtoKind::Ticket)
            .collect();
        let graph = TicketGraph::new(tickets.iter().map(|row| targets.stored_node(row)).collect());
        Self {
            targets,
            tickets,
            graph,
        }
    }

    /// The same for a read that returns no ticket: what every ID names,
    /// and a graph of nothing, which no document is looked up in.
    fn for_documents(rows: &[&'a StoredItem], comments: &'a [String]) -> Self {
        Self {
            targets: Targets::of(rows, comments),
            tickets: Vec::new(),
            graph: TicketGraph::new(Vec::new()),
        }
    }

    /// The graph with one ticket as its file has it now, in place of what
    /// the index stored for that ID, if it stored anything.
    fn graph_with(&self, ticket: TicketNode) -> TicketGraph {
        let mut nodes: Vec<TicketNode> = self
            .tickets
            .iter()
            .filter(|row| row.id != ticket.id)
            .map(|row| self.targets.stored_node(row))
            .collect();
        nodes.push(ticket);
        TicketGraph::new(nodes)
    }
}

/// The ID of every comment the index holds for the registration. They are
/// only compared with IDs that were checked, so none is checked here.
pub(super) fn stored_comment_ids(
    connection: &Connection,
    repo: &ResolvedRepository,
) -> Result<Vec<String>, ReadError> {
    let mut statement = connection.prepare(
        "SELECT comments.comment_id
           FROM discovered_comments AS comments
           JOIN discovered_items AS items ON items.id = comments.item_id
           JOIN contexts ON contexts.id = items.context_id
          WHERE contexts.repository_id = ?1",
    )?;
    let ids = statement
        .query_map([repo.registration_id()], |row| row.get(0))?
        .collect::<Result<_, _>>()?;
    Ok(ids)
}

/// A problem row that has both a path and a context.
pub(super) struct StoredProblem {
    pub(super) context: StoredContext,
    pub(super) path: String,
    pub(super) code: ProblemCode,
}

/// What an index row holds that is not what a read expects. No read
/// repairs or skips such a row.
pub(super) fn invalid_stored_data() -> ReadError {
    ReadError::new(ResultCode::InternalError)
}

fn stored_context_kind(stored: &str) -> Result<ItemContextKind, ReadError> {
    match stored {
        "primary" => Ok(ItemContextKind::Primary),
        "unverified" => Ok(ItemContextKind::Unverified),
        "active" => Ok(ItemContextKind::Active),
        _ => Err(invalid_stored_data()),
    }
}

fn stored_change_source(stored: &str) -> Result<ChangeSource, ReadError> {
    match stored {
        "git" => Ok(ChangeSource::GitCommit),
        "filesystem" => Ok(ChangeSource::Uncommitted),
        _ => Err(invalid_stored_data()),
    }
}

fn timestamp(seconds: i64) -> Option<String> {
    OffsetDateTime::from_unix_timestamp(seconds)
        .ok()
        .and_then(timestamp_string)
}

/// The registration's index state and, in seconds, when it was last
/// refreshed. A registration removed since the repository was resolved is
/// `repository_not_registered`.
pub(super) fn stored_index_state(
    connection: &Connection,
    repo: &ResolvedRepository,
) -> Result<(IndexStateDto, Option<i64>), ReadError> {
    let stored: Option<(bool, Option<i64>, bool)> = connection
        .query_row(
            "SELECT refresh_required, refreshed_at,
                    EXISTS(SELECT 1 FROM contexts
                            WHERE contexts.repository_id = repositories.id)
               FROM repositories
              WHERE id = ?1 AND root_path = ?2",
            rusqlite::params![repo.registration_id(), repo.root().to_str()],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()?;
    let (refresh_required, refreshed_at, has_contexts) =
        stored.ok_or_else(|| ReadError::new(ResultCode::RepositoryNotRegistered))?;
    Ok((
        index_state(refresh_required, refreshed_at, has_contexts),
        refreshed_at,
    ))
}

/// Every context of the registration, in worktree order.
fn stored_contexts(
    connection: &Connection,
    repo: &ResolvedRepository,
) -> Result<Vec<StoredContext>, ReadError> {
    let mut statement = connection.prepare(
        "SELECT kind, branch, worktree_path, head_oid FROM contexts
          WHERE repository_id = ?1
          ORDER BY worktree_path ASC, id ASC",
    )?;
    let rows = statement
        .query_map([repo.registration_id()], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<String>>(3)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    rows.into_iter()
        .map(|(kind, branch, worktree, head_oid)| {
            Ok(StoredContext {
                kind: stored_context_kind(&kind)?,
                branch,
                worktree,
                head_oid,
            })
        })
        .collect()
}

/// Every document and ticket the index holds for the registration. The
/// order is the index's; each caller sorts what it returns.
pub(super) fn stored_items(
    connection: &Connection,
    repo: &ResolvedRepository,
) -> Result<Vec<StoredItem>, ReadError> {
    let mut statement = connection.prepare(
        "SELECT contexts.kind, contexts.branch, contexts.worktree_path, contexts.head_oid,
                items.item_id, items.kind, items.canonical_path, items.title,
                items.ticket_type, items.status, items.project, items.team,
                items.closed_at, items.closed_by, items.unknown_metadata,
                items.activity_at, items.activity_source, items.id, items.slug
           FROM discovered_items AS items
           JOIN contexts ON contexts.id = items.context_id
          WHERE contexts.repository_id = ?1
          ORDER BY items.id ASC",
    )?;
    let mut rows = statement.query([repo.registration_id()])?;
    let mut items = Vec::new();
    while let Some(row) = rows.next()? {
        let context_kind: String = row.get(0)?;
        let kind: String = row.get(5)?;
        let unknown: Option<String> = row.get(14)?;
        let activity_source: String = row.get(16)?;
        let kind = ItemDtoKind::from_stored(&kind).ok_or_else(invalid_stored_data)?;
        // Only what the grammar allows is stored, and only for a ticket.
        let slug: Option<String> = row.get(18)?;
        if slug
            .as_ref()
            .is_some_and(|slug| kind != ItemDtoKind::Ticket || !canonical::is_valid_slug(slug))
        {
            return Err(invalid_stored_data());
        }
        items.push(StoredItem {
            row_id: row.get(17)?,
            context: StoredContext {
                kind: stored_context_kind(&context_kind)?,
                branch: row.get(1)?,
                worktree: row.get(2)?,
                head_oid: row.get(3)?,
            },
            id: row.get(4)?,
            kind,
            path: row.get(6)?,
            title: row.get(7)?,
            ticket_type: row.get(8)?,
            status: row.get(9)?,
            project: row.get(10)?,
            team: row.get(11)?,
            closed_at: row.get(12)?,
            closed_by: row.get(13)?,
            // A row stored before the column existed holds nothing in it.
            // The migration that added the column marked the index for a
            // refresh, so the list that carries this says it is behind.
            unknown: match unknown {
                Some(stored) => {
                    let mut unknown =
                        UnknownMetadata::from_stored(&stored).ok_or_else(invalid_stored_data)?;
                    // An index written before a ticket's relationship keys
                    // were fields of their own stored them here.
                    if kind == ItemDtoKind::Ticket {
                        for key in canonical::RELATIONSHIP_KEYS {
                            unknown.values.remove(key);
                        }
                    }
                    unknown
                }
                None => UnknownMetadata::default(),
            },
            relationships: Relationships {
                slug,
                ..Default::default()
            },
            activity_at: row.get(15)?,
            change_source: stored_change_source(&activity_source)?,
        });
    }
    stored_relationships(connection, repo, &mut items)?;
    Ok(items)
}

/// The item stored in the row `row_id`, among `items` in row order.
fn item_with_row(items: &mut [StoredItem], row_id: i64) -> Result<&mut StoredItem, ReadError> {
    let position = items
        .binary_search_by_key(&row_id, |item| item.row_id)
        .map_err(|_| invalid_stored_data())?;
    Ok(&mut items[position])
}

/// Adds to each of `items`, which are in row order, the edges and the
/// relationship problems stored under its row.
///
/// A row is checked as it is read, in time that does not grow with the
/// number of edges a ticket already has. An edge belongs to a ticket and names
/// another item by a well-formed ID, a ticket has at most one parent, and a
/// problem's detail is an item ID or nothing: discovery stores nothing
/// else, so anything else fails the read. A problem code this build does
/// not know as a relationship problem is `unknown_problem`, and is given
/// no ID.
fn stored_relationships(
    connection: &Connection,
    repo: &ResolvedRepository,
    items: &mut [StoredItem],
) -> Result<(), ReadError> {
    let mut statement = connection.prepare(
        "SELECT edges.item_id, edges.target_id, edges.kind
           FROM item_edges AS edges
           JOIN discovered_items AS items ON items.id = edges.item_id
           JOIN contexts ON contexts.id = items.context_id
          WHERE contexts.repository_id = ?1
          ORDER BY edges.id ASC",
    )?;
    let mut rows = statement.query([repo.registration_id()])?;
    // The dependencies read so far, by item row. The index's own unique
    // constraint keeps an edge from being stored twice; a stored row is
    // checked all the same.
    let mut seen: BTreeSet<(i64, String)> = BTreeSet::new();
    while let Some(row) = rows.next()? {
        let item = item_with_row(items, row.get(0)?)?;
        let target: String = row.get(1)?;
        let kind: String = row.get(2)?;
        if item.kind != ItemDtoKind::Ticket
            || target.parse::<ItemId>().is_err()
            || target == item.id
        {
            return Err(invalid_stored_data());
        }
        match kind.as_str() {
            "parent" if item.relationships.parent.is_none() => {
                item.relationships.parent = Some(target);
            }
            "deps" if seen.insert((item.row_id, target.clone())) => {
                item.relationships.deps.push(target);
            }
            _ => return Err(invalid_stored_data()),
        }
    }

    let mut statement = connection.prepare(
        "SELECT problems.item_id, problems.code, problems.detail
           FROM item_problems AS problems
           JOIN discovered_items AS items ON items.id = problems.item_id
           JOIN contexts ON contexts.id = items.context_id
          WHERE contexts.repository_id = ?1
          ORDER BY problems.id ASC",
    )?;
    let mut rows = statement.query([repo.registration_id()])?;
    while let Some(row) = rows.next()? {
        let item = item_with_row(items, row.get(0)?)?;
        let code: String = row.get(1)?;
        let detail: Option<String> = row.get(2)?;
        if item.kind != ItemDtoKind::Ticket
            || detail
                .as_ref()
                .is_some_and(|detail| detail.parse::<ItemId>().is_err())
        {
            return Err(invalid_stored_data());
        }
        let code = ProblemCode::from_stored(&code);
        item.relationships
            .problems
            .push(if RELATIONSHIP_PROBLEMS.contains(&code) {
                (code, detail)
            } else {
                (ProblemCode::UnknownProblem, None)
            });
    }
    Ok(())
}

/// Every stored problem that has a path and a context, in context, path,
/// code and then insertion order. The stored guidance is not read: it can
/// hold a parser's or the operating system's own words.
pub(super) fn stored_problems(
    connection: &Connection,
    repo: &ResolvedRepository,
) -> Result<Vec<StoredProblem>, ReadError> {
    let mut statement = connection.prepare(
        "SELECT contexts.kind, contexts.branch, contexts.worktree_path, contexts.head_oid,
                problems.path, problems.code
           FROM problems
           JOIN contexts ON contexts.id = problems.context_id
          WHERE problems.repository_id = ?1 AND problems.path IS NOT NULL
          ORDER BY contexts.worktree_path ASC, problems.path ASC, problems.code ASC,
                   problems.id ASC",
    )?;
    let mut rows = statement.query([repo.registration_id()])?;
    let mut problems = Vec::new();
    while let Some(row) = rows.next()? {
        let context_kind: String = row.get(0)?;
        let code: String = row.get(5)?;
        problems.push(StoredProblem {
            context: StoredContext {
                kind: stored_context_kind(&context_kind)?,
                branch: row.get(1)?,
                worktree: row.get(2)?,
                head_oid: row.get(3)?,
            },
            path: row.get(4)?,
            code: ProblemCode::from_stored(&code),
        });
    }
    Ok(problems)
}

pub(super) fn problem(code: ProblemCode, path: &str) -> ProblemDto {
    ProblemDto {
        code,
        path: Some(path.to_owned()),
        target_id: None,
    }
}

/// Puts tickets in the ticket list ordering: content-change time, latest
/// first, and then ID.
pub(super) fn ticket_list_order(tickets: &mut [&StoredItem]) {
    tickets.sort_by(|left, right| {
        (Reverse(left.activity_at), &left.id).cmp(&(Reverse(right.activity_at), &right.id))
    });
}

/// An item DTO that says where a file is and nothing about what it holds.
/// This is the whole of a nonconforming entry, and what a conforming item
/// is filled in from.
fn bare_item(
    kind: ItemDtoKind,
    path: &str,
    context: &StoredContext,
    problems: Vec<ProblemDto>,
    index: &IndexStateDto,
) -> ItemDto {
    ItemDto {
        id: None,
        kind,
        path: path.to_owned(),
        title: None,
        ticket_type: None,
        status: None,
        project: None,
        team: None,
        closure: None,
        slug: None,
        parent: None,
        deps: Vec::new(),
        readiness: None,
        unknown_metadata: serde_json::Map::new(),
        body: None,
        source: None,
        observation: None,
        context: context.dto(),
        changed_at: None,
        change_source: None,
        problems,
        index: index.clone(),
    }
}

fn closure(closed_at: Option<String>, closed_by: Option<String>) -> ClosureDto {
    ClosureDto {
        state: match closed_at {
            Some(_) => ClosureState::Closed,
            None => ClosureState::Open,
        },
        closed_at,
        closed_by,
    }
}

/// The problems an item's own metadata gives it.
pub(super) fn metadata_problems(unknown: &UnknownMetadata, path: &str) -> Vec<ProblemDto> {
    unknown
        .not_representable
        .then(|| problem(ProblemCode::MetadataNotRepresentable, path))
        .into_iter()
        .collect()
}

/// Fills in a ticket's `slug`, `parent`, `deps` and `readiness`, and adds
/// the problems of its relationship values after the ones it already has.
///
/// The parent and each dependency say what their target is among what the
/// index holds now: an open ticket, a closed one, or `unresolved` when no
/// context holds anything with that ID. A `deps` entry or a `parent` that
/// names a document or a comment names no ticket: it is left out and
/// reported with the ID. A problem the item already carries, about the
/// same ID, is not repeated.
///
/// `graph` holds the ticket as `relationships` has it. It gives the
/// readiness; `dependency_cycle`, with the lowest ID of the cycle, for
/// every ticket on a dependency cycle, closed ones and ones the cycle does
/// not block included; and `parent_cycle`, with the parent's ID, for a
/// ticket whose `parent` links lead back to it. Such a ticket is a root,
/// and is still shown the parent its file names. A document is no ticket
/// of the graph and gets none of these.
///
/// Every ID here was parsed as one, from the file or from the index, so a
/// problem's `target_id` is never text taken from front matter.
fn relate(
    dto: &mut ItemDto,
    relationships: &Relationships,
    targets: &Targets<'_>,
    graph: &TicketGraph,
) {
    let mut problems = relationships.problems.clone();
    let mut target = |id: &String| {
        let state = targets.state(id);
        if state.is_none() {
            problems.push((ProblemCode::RelationshipNotATicket, Some(id.clone())));
        }
        state.map(|state| DependencyDto {
            id: id.clone(),
            state,
        })
    };
    dto.slug = relationships.slug.clone();
    dto.parent = relationships.parent.as_ref().and_then(&mut target);
    dto.deps = relationships.deps.iter().filter_map(&mut target).collect();
    if let Some(id) = &dto.id {
        dto.readiness = graph.readiness(id);
        if let Some(lowest) = graph.dependency_cycle(id) {
            problems.push((ProblemCode::DependencyCycle, Some(lowest.to_owned())));
        }
        if graph.on_parent_cycle(id) {
            problems.push((ProblemCode::ParentCycle, relationships.parent.clone()));
        }
    }
    for (code, target_id) in problems {
        let problem = ProblemDto {
            target_id,
            ..problem(code, &dto.path)
        };
        if !dto.problems.contains(&problem) {
            dto.problems.push(problem);
        }
    }
}

/// An item as the index stored it: the list form.
pub(super) fn stored_item_dto(
    item: &StoredItem,
    related: &Related<'_>,
    index: &IndexStateDto,
) -> ItemDto {
    let is_ticket = item.kind == ItemDtoKind::Ticket;
    let mut dto = ItemDto {
        id: Some(item.id.clone()),
        title: Some(item.title.clone()),
        ticket_type: item.ticket_type.clone(),
        status: item.status.clone(),
        project: item.project.clone(),
        team: item.team.clone(),
        closure: is_ticket
            .then(|| closure(item.closed_at.and_then(timestamp), item.closed_by.clone())),
        unknown_metadata: item.unknown.values.clone(),
        changed_at: timestamp(item.activity_at),
        change_source: Some(item.change_source),
        problems: metadata_problems(&item.unknown, &item.path),
        ..bare_item(item.kind, &item.path, &item.context, Vec::new(), index)
    };
    relate(
        &mut dto,
        &item.relationships,
        &related.targets,
        &related.graph,
    );
    dto
}

/// A parsed file as an item with no index row behind it. `None` for a
/// comment, which is not an item.
///
/// A ticket's readiness is decided from its file: its own closure and
/// dependencies as they are there, against what the index holds of every
/// other ticket.
fn parsed_item_dto(
    item: canonical::CanonicalItem,
    path: &str,
    context: &StoredContext,
    index: &IndexStateDto,
    related: &Related<'_>,
) -> Option<ItemDto> {
    let (kind, id, title, unknown, body) = match &item {
        canonical::CanonicalItem::Document(document) => (
            ItemDtoKind::Document,
            &document.id,
            &document.title,
            UnknownMetadata::from_yaml(&document.unknown),
            &document.body,
        ),
        // The relationship keys are fields of their own, not unknown.
        canonical::CanonicalItem::Ticket(ticket) => (
            ItemDtoKind::Ticket,
            &ticket.id,
            &ticket.title,
            UnknownMetadata::from_ticket(ticket),
            &ticket.body,
        ),
        canonical::CanonicalItem::Comment(_) => return None,
    };
    let mut dto = ItemDto {
        id: Some(id.to_string()),
        title: Some(title.clone()),
        body: Some(body.clone()),
        problems: metadata_problems(&unknown, path),
        unknown_metadata: unknown.values,
        ..bare_item(kind, path, context, Vec::new(), index)
    };
    if let canonical::CanonicalItem::Ticket(ticket) = item {
        let relationships = Relationships::from(&canonical::ticket_relationships(&ticket));
        let targets = &related.targets;
        let graph = related.graph_with(targets.node(
            &ticket.id.to_string(),
            ticket.closed_at.is_some(),
            &relationships,
        ));
        relate(&mut dto, &relationships, targets, &graph);
        dto.closure = Some(closure(
            ticket.closed_at.and_then(timestamp_string),
            ticket.closed_by,
        ));
        dto.ticket_type = Some(ticket.ticket_type);
        dto.status = Some(ticket.status);
        dto.project = ticket.project;
        dto.team = ticket.team;
    }
    Some(dto)
}

/// Whether what the file holds is what the index stored for it, as far as
/// the index stores it.
fn same_indexed_metadata(file: &ItemDto, stored: &ItemDto) -> bool {
    file.id == stored.id
        && file.kind == stored.kind
        && file.title == stored.title
        && file.ticket_type == stored.ticket_type
        && file.status == stored.status
        && file.project == stored.project
        && file.team == stored.team
        && file.closure == stored.closure
        && file.slug == stored.slug
        && file.parent == stored.parent
        && file.deps == stored.deps
        && file.unknown_metadata == stored.unknown_metadata
        && file.problems == stored.problems
}

/// `index`, or `stale` when it claims to be current.
pub(super) fn behind(index: &IndexStateDto) -> IndexStateDto {
    IndexStateDto {
        state: match index.state {
            IndexState::Current => IndexState::Stale,
            state => state,
        },
        refreshed_at: index.refreshed_at.clone(),
    }
}

/// The observation token of a file: `v1:` and the lowercase hexadecimal
/// BLAKE3 digest of these bytes, in this order:
///
/// 1. one byte, `0x01` when the context has a branch and `0x00` when it has
///    none; and, only when it has one, the branch name's length in bytes as
///    an unsigned 64-bit little-endian integer followed by the name;
/// 2. the repository-relative path's length in bytes, encoded the same
///    way, followed by the path with forward slashes;
/// 3. the file's length in bytes, encoded the same way, followed by the
///    file's bytes as they are on disk.
///
/// The branch is the one the result's `context.branch` reports. Each part
/// is length-prefixed so that no two different triples give the same input.
pub(super) fn observation_token(branch: Option<&str>, path: &str, source: &[u8]) -> String {
    fn part(hasher: &mut blake3::Hasher, bytes: &[u8]) {
        hasher.update(&(bytes.len() as u64).to_le_bytes());
        hasher.update(bytes);
    }
    let mut hasher = blake3::Hasher::new();
    match branch {
        Some(branch) => {
            hasher.update(&[1]);
            part(&mut hasher, branch.as_bytes());
        }
        None => {
            hasher.update(&[0]);
        }
    }
    part(&mut hasher, path.as_bytes());
    part(&mut hasher, source);
    format!("v1:{}", hasher.finalize().to_hex())
}

/// An item file as it is on disk.
pub(super) struct ItemFile {
    pub(super) bytes: Vec<u8>,
    /// When it was last modified, in whole seconds, where the platform
    /// says.
    modified: Option<i64>,
}

/// What reading an item file found, short of a failure of the read itself.
pub(super) enum ItemFileRead {
    Found(ItemFile),
    /// Nothing is at the path.
    Missing,
    /// Something is at the path that is not a regular file reached through
    /// real directories: a symbolic link, a directory, a pipe, a device.
    NotAFile,
}

/// Where a context's worktree is, relative to the repository root: nowhere
/// for the root itself, and `.manyhands/worktrees/<item id>` for an item
/// worktree. Discovery records no other place, so any other is `None`.
fn context_directory(repo: &ResolvedRepository, worktree: &str) -> Option<PathBuf> {
    let relative = Path::new(worktree).strip_prefix(repo.root()).ok()?;
    if relative.as_os_str().is_empty() {
        return Some(PathBuf::new());
    }
    let id = relative.strip_prefix(WORKTREES_DIRECTORY).ok()?.to_str()?;
    id.parse::<ItemId>().ok()?;
    Some(Path::new(WORKTREES_DIRECTORY).join(id))
}

/// `root` followed by each component of `relative`, which is written with
/// forward slashes, so that the result uses the platform's own separator.
fn below(root: &Path, relative: &Path) -> PathBuf {
    relative
        .components()
        .fold(root.to_owned(), |path, component| path.join(component))
}

/// Whether `path` is written as a path inside a context: relative, with
/// forward slashes, with no NUL, no empty, `.` or `..` component, and not
/// under the directory that holds item worktrees. This says nothing about whether it
/// is a place an item can be.
pub(super) fn is_plain_relative(path: &str) -> bool {
    !path.is_empty()
        && !path.contains(['\\', '\0'])
        && path
            .split('/')
            .all(|segment| !segment.is_empty() && segment != "." && segment != "..")
        && Path::new(path)
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
        && !Path::new(path).starts_with(WORKTREES_DIRECTORY)
}

/// The kind of item `path` is the canonical place of, if any:
/// `docs/**/*.md` or `.manyhands/tickets/<id>/ticket.md`.
fn canonical_item_kind(path: &str) -> Option<ItemDtoKind> {
    match canonical::item_path_kind(Path::new(path)) {
        Ok(canonical::ItemKind::Document) => Some(ItemDtoKind::Document),
        Ok(canonical::ItemKind::Ticket) => Some(ItemDtoKind::Ticket),
        Ok(canonical::ItemKind::Comment) | Err(_) => None,
    }
}

/// Reads the file at `path` in the context whose worktree is `worktree`,
/// following no symbolic link below the repository root.
///
/// Both come from the index or from a caller, so both are checked first:
/// the worktree must be the root or an item worktree under it, and the
/// path a plain relative one. A row that is neither is not read from. The
/// caller decides which plain paths it reads.
///
/// The bytes come from the guarded reader, given the repository root and
/// the whole path below it, so that it opens every directory on the way,
/// the worktree's own included, and the file itself without following
/// links. What it cannot open for any reason other than what is at the
/// path, such as a file this user may not read, is
/// `repository_inaccessible`.
pub(super) fn read_item_file(
    repo: &ResolvedRepository,
    worktree: &str,
    path: &str,
) -> Result<ItemFileRead, ReadError> {
    let directory = context_directory(repo, worktree).ok_or_else(invalid_stored_data)?;
    if !is_plain_relative(path) {
        return Err(invalid_stored_data());
    }
    match guarded_item_file(repo.root(), &directory.join(path)) {
        Ok(GuardedFile::Found { bytes, modified }) => Ok(ItemFileRead::Found(ItemFile {
            bytes,
            modified: modified
                .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
                .and_then(|modified| i64::try_from(modified.as_secs()).ok()),
        })),
        Ok(GuardedFile::Missing) => Ok(ItemFileRead::Missing),
        Ok(GuardedFile::NotAFile) => Ok(ItemFileRead::NotAFile),
        Err(error) => Err(ReadError::new(ResultCode::RepositoryInaccessible).with_source(error)),
    }
}

#[cfg(unix)]
fn guarded_item_file(root: &Path, path: &Path) -> io::Result<GuardedFile> {
    crate::repository::guarded_file(root, path)
}

// Weaker than the Unix reader, and never compiled or executed here: making
// it sound and proving it on its platforms is a native obligation.
//
// The Unix reader opens each directory through the one before it without
// following links and inspects the descriptor it opened. This one checks
// the path and then uses the path: it looks at what is there, compares the
// file's real location with the path, and only then reads. Something that
// replaces the file or a directory between those steps is read. It can
// also wait on a file that is not a regular one if it is swapped in after
// the check.
#[cfg(not(unix))]
fn guarded_item_file(root: &Path, path: &Path) -> io::Result<GuardedFile> {
    let file = below(root, path);
    let metadata = match fs::symlink_metadata(&file) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(GuardedFile::Missing),
        Err(error) => return Err(error),
    };
    // The root is canonical, so a file reached through no link is where its
    // path says.
    if !metadata.file_type().is_file() || fs::canonicalize(&file)? != file {
        return Ok(GuardedFile::NotAFile);
    }
    Ok(GuardedFile::Found {
        bytes: fs::read(&file)?,
        modified: metadata.modified().ok(),
    })
}

impl ItemFile {
    /// Whether the file was modified in a later second than the one the
    /// last completed refresh began observing in.
    ///
    /// That time is taken before the refresh looks at anything, so a file
    /// changed while a refresh runs is newer. This still misses a change
    /// made in the same second the refresh began, a change that keeps or
    /// sets back the file's modification time, and any change where the
    /// platform reports no time; and a file whose time is in the future
    /// reads as newer until that time passes. A change to metadata the
    /// index stores is seen separately, by comparing it.
    pub(super) fn newer_than(&self, refreshed_at: Option<i64>) -> bool {
        self.modified
            .zip(refreshed_at)
            .is_some_and(|(modified, refreshed)| modified > refreshed)
    }
}

/// What a file is, once read: an item, or the reason it is not one.
enum ParsedFile {
    Item {
        dto: Box<ItemDto>,
        source: String,
    },
    Nonconforming {
        code: ProblemCode,
        source: Option<String>,
    },
}

fn parse_file(
    bytes: Vec<u8>,
    path: &str,
    context: &StoredContext,
    index: &IndexStateDto,
    related: &Related<'_>,
) -> ParsedFile {
    let Ok(source) = String::from_utf8(bytes) else {
        return ParsedFile::Nonconforming {
            code: ProblemCode::SourceUnreadable,
            source: None,
        };
    };
    // The problem's message can repeat the file's text; only its code is
    // kept.
    match canonical::parse_item(Path::new(path), &source) {
        Ok(item) => match parsed_item_dto(item, path, context, index, related) {
            Some(dto) => ParsedFile::Item {
                dto: Box::new(dto),
                source,
            },
            None => ParsedFile::Nonconforming {
                code: ProblemCode::KindPathMismatch,
                source: Some(source),
            },
        },
        Err(problem) => ParsedFile::Nonconforming {
            code: ProblemCode::from(&problem.code),
            source: Some(source),
        },
    }
}

/// Whether the context's worktree is still a directory. Only asked when the
/// index holds one item twice.
fn context_exists(repo: &ResolvedRepository, context: &StoredContext) -> bool {
    context_directory(repo, &context.worktree).is_some_and(|directory| {
        fs::symlink_metadata(below(repo.root(), &directory))
            .is_ok_and(|metadata| metadata.file_type().is_dir())
    })
}

/// Whether the row is the item's copy in the worktree created to edit that
/// item: an active context at `.manyhands/worktrees/<the item's own ID>`.
/// An item worktree's checkout holds every other item too, and a row for
/// one of those is never the effective copy.
fn is_own_worktree_row(repo: &ResolvedRepository, row: &StoredItem) -> bool {
    row.context.kind == ItemContextKind::Active
        && context_directory(repo, &row.context.worktree)
            .is_some_and(|directory| directory == Path::new(WORKTREES_DIRECTORY).join(&row.id))
}

const OWN_WORKTREE: u8 = 0;
const OTHER_WORKTREE: u8 = 2;

/// The order in which the rows of one item are tried: its own worktree's,
/// then one outside any item worktree, then a copy in another item's.
///
/// That last kind is never the effective copy and is never preferred. It is
/// still answered from when the index offers nothing better, as a last
/// resort, because it is the only place the index knows the item to be; the
/// index is then reported as behind.
fn row_rank(repo: &ResolvedRepository, row: &StoredItem) -> u8 {
    if is_own_worktree_row(repo, row) {
        OWN_WORKTREE
    } else if row.context.kind != ItemContextKind::Active {
        1
    } else {
        OTHER_WORKTREE
    }
}

/// One row for each item, and whether the index is known to be behind: it
/// held an item more than once, or held one only as a copy in another
/// item's worktree.
///
/// A refresh stores the root, each item worktree and the removal of
/// worktrees that are gone in separate transactions, so a read can find an
/// item both in the primary context and in the row of a worktree that no
/// longer exists, or the reverse. Then the row of the item's own worktree
/// is the effective one if that worktree is still there, and otherwise the
/// primary one; and the index, which is in the middle of changing, is
/// behind.
pub(super) fn effective_rows<'a>(
    repo: &ResolvedRepository,
    stored: &'a [StoredItem],
) -> (Vec<&'a StoredItem>, bool) {
    let mut by_id: BTreeMap<&str, Vec<&StoredItem>> = BTreeMap::new();
    for item in stored {
        by_id.entry(&item.id).or_default().push(item);
    }
    let mut is_behind = false;
    let rows = by_id
        .into_values()
        .map(|mut rows| {
            is_behind |= rows.len() > 1;
            rows.sort_by_key(|row| row_rank(repo, row));
            let row = rows
                .iter()
                .find(|row| {
                    rows.len() == 1
                        || row_rank(repo, row) != OWN_WORKTREE
                        || context_exists(repo, &row.context)
                })
                .copied()
                .unwrap_or(rows[0]);
            is_behind |= row_rank(repo, row) == OTHER_WORKTREE;
            row
        })
        .collect();
    (rows, is_behind)
}

/// The effective copy of one item: the row the index holds it in, the file
/// that row names as it is now, and the index state to report with it.
pub(super) struct EffectiveCopy<'a> {
    pub(super) row: &'a StoredItem,
    pub(super) file: ItemFile,
    pub(super) index: IndexStateDto,
}

/// Finds the effective copy of the item `id` among the stored rows and
/// reads its file.
///
/// The rows are tried in `row_rank` order: the item's own worktree first,
/// then the primary copy, and a copy in another item's worktree last. Only
/// a row with nothing behind it gives way to the next; a copy that is there
/// and cannot be read is a failure, not a reason to answer from another
/// copy. `index` comes back `stale` when it claimed to be current and held
/// the item more than once, or only as a copy in another item's worktree.
///
/// No row, and no row with a file behind it, is `item_not_found`.
pub(super) fn effective_copy<'a>(
    repo: &ResolvedRepository,
    stored: &'a [StoredItem],
    id: &str,
    index: IndexStateDto,
) -> Result<EffectiveCopy<'a>, ReadError> {
    let mut rows: Vec<&StoredItem> = stored.iter().filter(|item| item.id == id).collect();
    // A refresh can only find the item if the index is behind.
    if rows.is_empty() {
        return Err(item_not_found(repo, index.state != IndexState::Current));
    }
    let index = if rows.len() > 1 {
        behind(&index)
    } else {
        index
    };
    // The sort keeps the index's order within each rank.
    rows.sort_by_key(|row| row_rank(repo, row));
    for row in rows {
        if canonical_item_kind(&row.path).is_none() {
            return Err(invalid_stored_data());
        }
        match read_item_file(repo, &row.context.worktree, &row.path)? {
            ItemFileRead::Found(file) => {
                // Answered from a copy in another item's worktree: a last
                // resort, and the index is behind for offering nothing
                // better.
                let index = if row_rank(repo, row) == OTHER_WORKTREE {
                    behind(&index)
                } else {
                    index
                };
                return Ok(EffectiveCopy { row, file, index });
            }
            ItemFileRead::Missing | ItemFileRead::NotAFile => {}
        }
    }
    Err(item_not_found(repo, true))
}

impl RepositoryService {
    /// Every managed document, ordered by path and then ID, with a
    /// nonconforming entry for each file under `docs/` that is not one.
    ///
    /// The list is what the index holds; `index` says how far behind the
    /// repository that is. No file is opened.
    pub fn list_documents(&self, repo: &ResolvedRepository) -> Result<ItemListDto, ReadError> {
        self.list_items(repo, ItemDtoKind::Document, None)
    }

    /// The tickets `filter` matches, ordered by content-change time, latest
    /// first, and then ID. Each says whether it is ready, blocked or closed.
    /// A nonconforming entry for each file under `.manyhands/tickets/` that
    /// is not a ticket follows them, in path order, whatever the filter.
    ///
    /// The list is what the index holds; `index` says how far behind the
    /// repository that is. No file is opened.
    pub fn list_tickets(
        &self,
        repo: &ResolvedRepository,
        filter: &TicketFilter,
    ) -> Result<ItemListDto, ReadError> {
        self.list_items(repo, ItemDtoKind::Ticket, Some(filter))
    }

    fn list_items(
        &self,
        repo: &ResolvedRepository,
        kind: ItemDtoKind,
        filter: Option<&TicketFilter>,
    ) -> Result<ItemListDto, ReadError> {
        self.read_session(RepositoryOperation::Read, |connection| {
            let (index, _) = stored_index_state(connection, repo)?;
            let stored = stored_items(connection, repo)?;
            let problems = stored_problems(connection, repo)?;
            let (rows, is_behind) = effective_rows(repo, &stored);
            let index = if is_behind { behind(&index) } else { index };
            let comments = stored_comment_ids(connection, repo)?;
            let related = match kind {
                ItemDtoKind::Document => Related::for_documents(&rows, &comments),
                ItemDtoKind::Ticket => Related::of(&rows, &comments),
            };

            let mut listed: Vec<&StoredItem> = rows
                .into_iter()
                .filter(|item| item.kind == kind)
                .filter(|item| filter.is_none_or(|filter| filter.matches(item, &related.graph)))
                .collect();
            let mut nonconforming = nonconforming_entries(kind, &stored, &problems, &index);
            let items = match kind {
                ItemDtoKind::Document => {
                    let mut items: Vec<ItemDto> = listed
                        .into_iter()
                        .map(|item| stored_item_dto(item, &related, &index))
                        .chain(nonconforming)
                        .collect();
                    // A nonconforming entry has no ID and sorts before an
                    // item at the same path; the worktree settles the rest.
                    items.sort_by(|left, right| {
                        (&left.path, &left.id, &left.context.worktree).cmp(&(
                            &right.path,
                            &right.id,
                            &right.context.worktree,
                        ))
                    });
                    items
                }
                ItemDtoKind::Ticket => {
                    ticket_list_order(&mut listed);
                    nonconforming.sort_by(|left, right| {
                        (&left.path, &left.context.worktree)
                            .cmp(&(&right.path, &right.context.worktree))
                    });
                    listed
                        .into_iter()
                        .map(|item| stored_item_dto(item, &related, &index))
                        .chain(nonconforming)
                        .collect()
                }
            };
            Ok(ItemListDto {
                items,
                complete: true,
                index,
            })
        })
        .map_err(|error| repo.failure(error))
    }

    /// The item with this ID, read from its effective copy: the worktree
    /// created to edit it when there is one, otherwise the primary copy.
    /// `context` says which.
    ///
    /// The index says where the item is; everything else comes from the
    /// file as it is now. When that is no longer what the index stored,
    /// `index.state` is `stale`. A file that is gone, is no longer a
    /// regular file or now holds another item is `item_not_found`, with a
    /// refresh as the recovery. A file that no longer parses is returned in
    /// the nonconforming form with its source. A file that cannot be opened
    /// is `repository_inaccessible`.
    ///
    /// When the index holds the item in more than one context, which it can
    /// while a refresh is under way, the copy in the item's own worktree is
    /// read if it is still there, otherwise the primary copy, and the index
    /// is `stale`. A copy that is there and cannot be read fails the read.
    pub fn show_item(&self, repo: &ResolvedRepository, id: &ItemId) -> Result<ItemDto, ReadError> {
        let id = id.to_string();
        self.read_session(RepositoryOperation::Read, |connection| {
            let (index, refreshed_at) = stored_index_state(connection, repo)?;
            let stored = stored_items(connection, repo)?;
            let comments = stored_comment_ids(connection, repo)?;
            let related = Related::of(&effective_rows(repo, &stored).0, &comments);
            let EffectiveCopy { row, file, index } = effective_copy(repo, &stored, &id, index)?;
            let observation =
                observation_token(row.context.branch.as_deref(), &row.path, &file.bytes);
            let newer = file.newer_than(refreshed_at);
            let listed = stored_item_dto(row, &related, &index);
            let mut item = match parse_file(file.bytes, &row.path, &row.context, &index, &related) {
                ParsedFile::Item { dto, source } => {
                    // Another item is now where this one was.
                    if dto.id != listed.id || dto.kind != listed.kind {
                        return Err(item_not_found(repo, true));
                    }
                    let mut item = *dto;
                    if newer || !same_indexed_metadata(&item, &listed) {
                        item.index = behind(&index);
                    }
                    item.changed_at = listed.changed_at;
                    item.change_source = listed.change_source;
                    item.source = Some(source);
                    item
                }
                // The index holds an item here, and the file is not one.
                ParsedFile::Nonconforming { code, source } => ItemDto {
                    source,
                    ..bare_item(
                        row.kind,
                        &row.path,
                        &row.context,
                        vec![problem(code, &row.path)],
                        &behind(&index),
                    )
                },
            };
            item.observation = Some(observation);
            Ok(item)
        })
        .map_err(|error| {
            let mut error = repo.failure(error);
            error.scope.item_id = Some(id.clone());
            error
        })
    }

    /// The file at exactly `path`, whether or not it is an item: the way to
    /// read a nonconforming entry, which has no ID.
    ///
    /// `path` is relative to the context, written with forward slashes and
    /// with no empty, `.` or `..` component, and not under
    /// `.manyhands/worktrees`. It must be either a canonical item path,
    /// `docs/**/*.md` or `.manyhands/tickets/<id>/ticket.md`, or the path
    /// of a nonconforming entry the lists show for that context: one under
    /// `docs/` or `.manyhands/tickets/` for which the index holds a
    /// conformity problem. Anything else is `invalid_path` and nothing is
    /// read. So is a path that reaches the file through a symbolic link, or
    /// names something that is not a regular file. Nothing at the path is
    /// `path_not_found`, and a file that cannot be opened is
    /// `repository_inaccessible`.
    ///
    /// `context` is the repository root, or the worktree of one of its
    /// item contexts as the index has it; `None` is the root. Any other
    /// path is `invalid_path`. Reading by path does not choose the
    /// effective copy: it reads the copy in the context that was named.
    pub fn show_path(
        &self,
        repo: &ResolvedRepository,
        context: Option<&Path>,
        path: &Path,
    ) -> Result<ItemDto, ReadError> {
        self.read_path(repo, context, path)
            .map_err(|error| repo.failure(error))
    }

    fn read_path(
        &self,
        repo: &ResolvedRepository,
        context: Option<&Path>,
        path: &Path,
    ) -> Result<ItemDto, ReadError> {
        let path = path
            .to_str()
            .filter(|path| is_plain_relative(path))
            .ok_or_else(ReadError::invalid_path)?;
        let root = absolute_path_string(repo.root()).ok_or_else(ReadError::invalid_path)?;
        let worktree = match context {
            None => root.clone(),
            Some(context) => fs::canonicalize(context)
                .ok()
                .and_then(|context| absolute_path_string(&context))
                .ok_or_else(ReadError::invalid_path)?,
        };
        self.read_session(RepositoryOperation::Read, |connection| {
            let (index, refreshed_at) = stored_index_state(connection, repo)?;
            let context = match stored_contexts(connection, repo)?
                .into_iter()
                .find(|context| context.worktree == worktree)
            {
                Some(context) if worktree == root || context.kind == ItemContextKind::Active => {
                    context
                }
                Some(_) => return Err(ReadError::invalid_path()),
                // The root is a context whether or not it has been
                // observed; unobserved, nothing has verified its branch.
                None if worktree == root => StoredContext {
                    kind: ItemContextKind::Unverified,
                    branch: None,
                    worktree: root.clone(),
                    head_oid: None,
                },
                None => return Err(ReadError::invalid_path()),
            };
            // What the index holds for this file in this context.
            let here: Vec<ProblemCode> = stored_problems(connection, repo)?
                .into_iter()
                .filter(|problem| problem.context.worktree == worktree && problem.path == path)
                .map(|problem| problem.code)
                .collect();
            // A path that is not an item's is read only when a list shows
            // it: the same rule that makes the entry.
            let kind = canonical_item_kind(path)
                .or_else(|| {
                    ItemDtoKind::ALL
                        .into_iter()
                        .find(|kind| path.starts_with(kind.path_prefix()))
                        .filter(|_| here.iter().any(|code| CONFORMITY_PROBLEMS.contains(code)))
                })
                .ok_or_else(|| {
                    // An index that is behind may not yet hold the problem
                    // that would make this path one a list shows.
                    let error = ReadError::invalid_path();
                    if index.state == IndexState::Current {
                        error
                    } else {
                        error.with_recovery(vec![refresh_action(repo)])
                    }
                })?;
            let file = match read_item_file(repo, &context.worktree, path)? {
                ItemFileRead::Found(file) => file,
                ItemFileRead::Missing => return Err(ReadError::new(ResultCode::PathNotFound)),
                ItemFileRead::NotAFile => return Err(ReadError::invalid_path()),
            };
            let observation = observation_token(context.branch.as_deref(), path, &file.bytes);
            let newer = file.newer_than(refreshed_at);
            let stored = stored_items(connection, repo)?;
            let comments = stored_comment_ids(connection, repo)?;
            let related = Related::of(&effective_rows(repo, &stored).0, &comments);
            let row = stored
                .iter()
                .find(|item| item.context.worktree == worktree && item.path == path);
            let mut item = match parse_file(file.bytes, path, &context, &index, &related) {
                ParsedFile::Item { dto, source } => {
                    let mut item = *dto;
                    match row.map(|row| stored_item_dto(row, &related, &index)) {
                        Some(listed) if listed.id == item.id => {
                            if newer || !same_indexed_metadata(&item, &listed) {
                                item.index = behind(&index);
                            }
                            item.changed_at = listed.changed_at;
                            item.change_source = listed.change_source;
                        }
                        // The index holds another item at this path.
                        Some(_) => item.index = behind(&index),
                        None => {
                            // A file can parse and still not be an item of
                            // its context: another file there has its ID.
                            let duplicate = here.contains(&ProblemCode::DuplicateId);
                            if duplicate {
                                item.problems.push(problem(ProblemCode::DuplicateId, path));
                            }
                            // Otherwise, where no context holds this item
                            // the index has not seen it. Where another
                            // context does, this is a copy that is not the
                            // effective one, and the index is right not to
                            // list it here.
                            let known = duplicate
                                || stored
                                    .iter()
                                    .any(|other| Some(&other.id) == item.id.as_ref());
                            if newer || !known {
                                item.index = behind(&index);
                            }
                        }
                    }
                    item.source = Some(source);
                    item
                }
                ParsedFile::Nonconforming { code, source } => {
                    // Behind unless the index already holds this problem
                    // for this file as it is now.
                    let index = if row.is_none() && here.contains(&code) && !newer {
                        index.clone()
                    } else {
                        behind(&index)
                    };
                    ItemDto {
                        source,
                        ..bare_item(kind, path, &context, vec![problem(code, path)], &index)
                    }
                }
            };
            item.observation = Some(observation);
            Ok(item)
        })
    }
}

/// The nonconforming entries of a list of `kind`: one for each file, in
/// each context, that has a stored conformity problem, lies where an item
/// of that kind lives, and is not the file of an item the index holds in
/// that context. A file with several problems is one entry, and a problem
/// stored twice is listed once.
fn nonconforming_entries(
    kind: ItemDtoKind,
    stored: &[StoredItem],
    problems: &[StoredProblem],
    index: &IndexStateDto,
) -> Vec<ItemDto> {
    let mut entries: Vec<ItemDto> = Vec::new();
    for stored_problem in problems {
        if !CONFORMITY_PROBLEMS.contains(&stored_problem.code)
            || !stored_problem.path.starts_with(kind.path_prefix())
            || stored.iter().any(|item| {
                item.context.worktree == stored_problem.context.worktree
                    && item.path == stored_problem.path
            })
        {
            continue;
        }
        let dto = problem(stored_problem.code, &stored_problem.path);
        // The problems arrive in context and then path order.
        match entries.last_mut() {
            Some(entry)
                if entry.path == stored_problem.path
                    && entry.context.worktree == stored_problem.context.worktree =>
            {
                if !entry.problems.contains(&dto) {
                    entry.problems.push(dto);
                }
            }
            _ => entries.push(bare_item(
                kind,
                &stored_problem.path,
                &stored_problem.context,
                vec![dto],
                index,
            )),
        }
    }
    entries
}

/// `item_not_found`, with a refresh of the repository as its recovery when
/// a refresh could change the answer.
pub(super) fn item_not_found(repo: &ResolvedRepository, refresh: bool) -> ReadError {
    let error = ReadError::new(ResultCode::ItemNotFound);
    if !refresh {
        return error;
    }
    error.with_recovery(vec![refresh_action(repo)])
}

/// The recovery that refreshes this repository's index.
fn refresh_action(repo: &ResolvedRepository) -> RecoveryAction {
    let root = absolute_path_string(repo.root());
    root_action(REFRESH_INDEX_ACTION, root.as_deref())
}

#[cfg(test)]
#[path = "items_tests.rs"]
mod tests;
