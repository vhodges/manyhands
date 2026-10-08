//! Document and ticket lists, and complete reads of one item.
//!
//! A list is built from the index alone and opens no item file. A complete
//! read finds where the item is in the index and then reads that one file,
//! through a guarded reader that follows no symbolic link.

use std::{
    cmp::Reverse,
    collections::BTreeMap,
    fs, io,
    path::{Component, Path, PathBuf},
    time::UNIX_EPOCH,
};

use rusqlite::{Connection, OptionalExtension};
use time::OffsetDateTime;

use super::{
    ChangeSource, ClosureDto, ClosureState, IndexState, IndexStateDto, ItemContextDto,
    ItemContextKind, ItemDto, ItemDtoKind, ItemListDto, ProblemDto, REFRESH_INDEX_ACTION,
    ReadError, ResolvedRepository, index_state, root_action,
};
use crate::{
    canonical::{self, ItemId},
    repository::{GuardedFile, RepositoryOperation, RepositoryService, discovery::UnknownMetadata},
    results::{ProblemCode, ResultCode, absolute_path_string, timestamp_string},
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
/// filter matches every ticket. The default matches all of them.
///
/// A nonconforming entry has no metadata to match and is never removed.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TicketFilter {
    pub status: Option<String>,
    pub ticket_type: Option<String>,
    pub project: Option<String>,
    pub closure: ClosureFilter,
    /// Not applied yet. It takes effect when tickets' short codes are read
    /// (Task 8); until then a list is the same whatever this holds.
    pub slug: Option<String>,
    /// Not applied yet. It takes effect with the relationship queries
    /// (Task 9); until then a list is the same whatever this holds.
    pub readiness: Option<ReadinessFilter>,
}

impl TicketFilter {
    fn matches(&self, item: &StoredItem) -> bool {
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
struct StoredContext {
    kind: ItemContextKind,
    branch: Option<String>,
    worktree: String,
    head_oid: Option<String>,
}

impl StoredContext {
    fn dto(&self) -> ItemContextDto {
        ItemContextDto {
            kind: self.kind,
            branch: self.branch.clone(),
            worktree: self.worktree.clone(),
            head_oid: self.head_oid.clone(),
        }
    }
}

/// An item row with its context.
struct StoredItem {
    context: StoredContext,
    id: String,
    kind: ItemDtoKind,
    path: String,
    title: String,
    ticket_type: Option<String>,
    status: Option<String>,
    project: Option<String>,
    team: Option<String>,
    closed_at: Option<i64>,
    closed_by: Option<String>,
    unknown: UnknownMetadata,
    activity_at: i64,
    change_source: ChangeSource,
}

/// A problem row that has both a path and a context.
struct StoredProblem {
    context: StoredContext,
    path: String,
    code: ProblemCode,
}

/// What an index row holds that is not what a read expects. No read
/// repairs or skips such a row.
fn invalid_stored_data() -> ReadError {
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
fn stored_index_state(
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
fn stored_items(
    connection: &Connection,
    repo: &ResolvedRepository,
) -> Result<Vec<StoredItem>, ReadError> {
    let mut statement = connection.prepare(
        "SELECT contexts.kind, contexts.branch, contexts.worktree_path, contexts.head_oid,
                items.item_id, items.kind, items.canonical_path, items.title,
                items.ticket_type, items.status, items.project, items.team,
                items.closed_at, items.closed_by, items.unknown_metadata,
                items.activity_at, items.activity_source
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
        items.push(StoredItem {
            context: StoredContext {
                kind: stored_context_kind(&context_kind)?,
                branch: row.get(1)?,
                worktree: row.get(2)?,
                head_oid: row.get(3)?,
            },
            id: row.get(4)?,
            kind: ItemDtoKind::from_stored(&kind).ok_or_else(invalid_stored_data)?,
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
                    UnknownMetadata::from_stored(&stored).ok_or_else(invalid_stored_data)?
                }
                None => UnknownMetadata::default(),
            },
            activity_at: row.get(15)?,
            change_source: stored_change_source(&activity_source)?,
        });
    }
    Ok(items)
}

/// Every stored problem that has a path and a context, in context, path,
/// code and then insertion order. The stored guidance is not read: it can
/// hold a parser's or the operating system's own words.
fn stored_problems(
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

fn problem(code: ProblemCode, path: &str) -> ProblemDto {
    ProblemDto {
        code,
        path: Some(path.to_owned()),
    }
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
fn metadata_problems(unknown: &UnknownMetadata, path: &str) -> Vec<ProblemDto> {
    unknown
        .not_representable
        .then(|| problem(ProblemCode::MetadataNotRepresentable, path))
        .into_iter()
        .collect()
}

/// An item as the index stored it: the list form.
fn stored_item_dto(item: &StoredItem, index: &IndexStateDto) -> ItemDto {
    let is_ticket = item.kind == ItemDtoKind::Ticket;
    ItemDto {
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
    }
}

/// A parsed file as an item with no index row behind it. `None` for a
/// comment, which is not an item.
fn parsed_item_dto(
    item: canonical::CanonicalItem,
    path: &str,
    context: &StoredContext,
    index: &IndexStateDto,
) -> Option<ItemDto> {
    let (kind, id, title, unknown, body) = match &item {
        canonical::CanonicalItem::Document(document) => (
            ItemDtoKind::Document,
            &document.id,
            &document.title,
            &document.unknown,
            &document.body,
        ),
        canonical::CanonicalItem::Ticket(ticket) => (
            ItemDtoKind::Ticket,
            &ticket.id,
            &ticket.title,
            &ticket.unknown,
            &ticket.body,
        ),
        canonical::CanonicalItem::Comment(_) => return None,
    };
    let unknown = UnknownMetadata::from_yaml(unknown);
    let mut dto = ItemDto {
        id: Some(id.to_string()),
        title: Some(title.clone()),
        body: Some(body.clone()),
        problems: metadata_problems(&unknown, path),
        unknown_metadata: unknown.values,
        ..bare_item(kind, path, context, Vec::new(), index)
    };
    if let canonical::CanonicalItem::Ticket(ticket) = item {
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
        && file.unknown_metadata == stored.unknown_metadata
        && file.problems == stored.problems
}

/// `index`, or `stale` when it claims to be current.
fn behind(index: &IndexStateDto) -> IndexStateDto {
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
struct ItemFile {
    bytes: Vec<u8>,
    /// When it was last modified, in whole seconds, where the platform
    /// says.
    modified: Option<i64>,
}

/// What reading an item file found, short of a failure of the read itself.
enum ItemFileRead {
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
/// forward slashes, with no empty, `.` or `..` component, and not under the
/// directory that holds item worktrees. This says nothing about whether it
/// is a place an item can be.
fn is_plain_relative(path: &str) -> bool {
    !path.is_empty()
        && !path.contains('\\')
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
fn read_item_file(
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
    fn newer_than(&self, refreshed_at: Option<i64>) -> bool {
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
        Ok(item) => match parsed_item_dto(item, path, context, index) {
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

/// One row for each item, and whether the index held any item more than
/// once.
///
/// A refresh stores the root, each item worktree and the removal of
/// worktrees that are gone in separate transactions, so a read can find an
/// item both in the primary context and in the row of a worktree that no
/// longer exists, or the reverse. Then the item worktree's row is the
/// effective one if its worktree is still there, and otherwise the primary
/// one; and the index, which is in the middle of changing, is behind.
fn effective_rows<'a>(
    repo: &ResolvedRepository,
    stored: &'a [StoredItem],
) -> (Vec<&'a StoredItem>, bool) {
    let mut by_id: BTreeMap<&str, Vec<&StoredItem>> = BTreeMap::new();
    for item in stored {
        by_id.entry(&item.id).or_default().push(item);
    }
    let mut duplicated = false;
    let rows = by_id
        .into_values()
        .map(|rows| {
            if rows.len() == 1 {
                return rows[0];
            }
            duplicated = true;
            let is_active = |row: &&&StoredItem| row.context.kind == ItemContextKind::Active;
            rows.iter()
                .filter(is_active)
                .find(|row| context_exists(repo, &row.context))
                .or_else(|| rows.iter().find(|row| !is_active(row)))
                .copied()
                .unwrap_or(rows[0])
        })
        .collect();
    (rows, duplicated)
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
    /// first, and then ID. A nonconforming entry for each file under
    /// `.manyhands/tickets/` that is not a ticket follows them, in path
    /// order, whatever the filter.
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
            let (rows, duplicated) = effective_rows(repo, &stored);
            let index = if duplicated { behind(&index) } else { index };

            let mut listed: Vec<&StoredItem> = rows
                .into_iter()
                .filter(|item| item.kind == kind)
                .filter(|item| filter.is_none_or(|filter| filter.matches(item)))
                .collect();
            let mut nonconforming = nonconforming_entries(kind, &stored, &problems, &index);
            let items = match kind {
                ItemDtoKind::Document => {
                    let mut items: Vec<ItemDto> = listed
                        .into_iter()
                        .map(|item| stored_item_dto(item, &index))
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
                    listed.sort_by(|left, right| {
                        (Reverse(left.activity_at), &left.id)
                            .cmp(&(Reverse(right.activity_at), &right.id))
                    });
                    nonconforming.sort_by(|left, right| {
                        (&left.path, &left.context.worktree)
                            .cmp(&(&right.path, &right.context.worktree))
                    });
                    listed
                        .into_iter()
                        .map(|item| stored_item_dto(item, &index))
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
    /// while a refresh is under way, the item worktree's copy is read if it
    /// is still there, otherwise the primary copy, and the index is `stale`.
    pub fn show_item(&self, repo: &ResolvedRepository, id: &ItemId) -> Result<ItemDto, ReadError> {
        let id = id.to_string();
        self.read_session(RepositoryOperation::Read, |connection| {
            let (index, refreshed_at) = stored_index_state(connection, repo)?;
            let stored = stored_items(connection, repo)?;
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
            // An item worktree's row first. The sort keeps the index's order
            // among the rest.
            rows.sort_by_key(|row| row.context.kind != ItemContextKind::Active);
            let mut found = None;
            for (position, row) in rows.iter().enumerate() {
                let read = match canonical_item_kind(&row.path) {
                    Some(_) => read_item_file(repo, &row.context.worktree, &row.path),
                    None => Err(invalid_stored_data()),
                };
                match read {
                    Ok(ItemFileRead::Found(file)) => {
                        found = Some((*row, file));
                        break;
                    }
                    // A row with nothing behind it gives way to the next.
                    _ if position + 1 < rows.len() => {}
                    Ok(ItemFileRead::Missing | ItemFileRead::NotAFile) => {
                        return Err(item_not_found(repo, true));
                    }
                    Err(error) => return Err(error),
                }
            }
            let Some((row, file)) = found else {
                return Err(item_not_found(repo, true));
            };
            let observation =
                observation_token(row.context.branch.as_deref(), &row.path, &file.bytes);
            let newer = file.newer_than(refreshed_at);
            let listed = stored_item_dto(row, &index);
            let mut item = match parse_file(file.bytes, &row.path, &row.context, &index) {
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
                .ok_or_else(ReadError::invalid_path)?;
            let file = match read_item_file(repo, &context.worktree, path)? {
                ItemFileRead::Found(file) => file,
                ItemFileRead::Missing => return Err(ReadError::new(ResultCode::PathNotFound)),
                ItemFileRead::NotAFile => return Err(ReadError::invalid_path()),
            };
            let observation = observation_token(context.branch.as_deref(), path, &file.bytes);
            let newer = file.newer_than(refreshed_at);
            let stored = stored_items(connection, repo)?;
            let row = stored
                .iter()
                .find(|item| item.context.worktree == worktree && item.path == path);
            let mut item = match parse_file(file.bytes, path, &context, &index) {
                ParsedFile::Item { dto, source } => {
                    let mut item = *dto;
                    match row.map(|row| stored_item_dto(row, &index)) {
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
            || !is_plain_relative(&stored_problem.path)
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
fn item_not_found(repo: &ResolvedRepository, refresh: bool) -> ReadError {
    let error = ReadError::new(ResultCode::ItemNotFound);
    if !refresh {
        return error;
    }
    let root = absolute_path_string(repo.root());
    error.with_recovery(vec![root_action(REFRESH_INDEX_ACTION, root.as_deref())])
}

#[cfg(test)]
#[path = "items_tests.rs"]
mod tests;
