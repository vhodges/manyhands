//! Draft types and the version 1 file format.
//!
//! Types that hold draft text derive no `Debug`.

use std::{fmt, str::FromStr};

use serde::Deserialize;
use serde_json::{Map, Value, json};
use time::{OffsetDateTime, UtcOffset, format_description::well_known::Rfc3339};

use crate::canonical::ItemId;

const FORMAT: &str = "manyhands-draft";
const VERSION: u64 = 1;

/// The identity of one draft, and the name of its file.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub struct DraftId(ulid::Ulid);

impl DraftId {
    /// A new, random draft ID.
    pub fn generate() -> Self {
        Self(ulid::Ulid::new())
    }
}

impl fmt::Display for DraftId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

impl FromStr for DraftId {
    type Err = ();

    fn from_str(source: &str) -> Result<Self, Self::Err> {
        let id = ulid::Ulid::from_string(source).map_err(|_| ())?;
        // Only the canonical uppercase text names a draft.
        if id.to_string() == source {
            Ok(Self(id))
        } else {
            Err(())
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ItemKind {
    Document,
    Ticket,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct CommentTarget {
    pub comment_id: ItemId,
    pub parent_id: Option<ItemId>,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct DraftTarget {
    /// The registration root as the repository list returned it.
    pub repository: String,
    pub item_kind: ItemKind,
    pub item_id: ItemId,
    pub comment: Option<CommentTarget>,
}

#[derive(Clone, PartialEq, Eq)]
pub enum DraftFields {
    Document {
        title: String,
        body: String,
    },
    Ticket {
        title: String,
        ticket_type: String,
        status: String,
        project: Option<String>,
        team: Option<String>,
        deps: Vec<String>,
        parent: Option<String>,
        body: String,
    },
    Comment {
        body: String,
    },
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ContextKind {
    Primary,
    Unverified,
    Active,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct BaseContext {
    pub kind: ContextKind,
    pub branch: Option<String>,
    pub worktree: String,
    pub head_oid: Option<String>,
}

#[derive(Clone, PartialEq, Eq)]
pub struct DraftBase {
    /// `None` when the base is unverified.
    pub observation: Option<String>,
    pub path: String,
    pub context: BaseContext,
    /// `None` when the file was not UTF-8.
    pub source: Option<String>,
    pub fields: DraftFields,
}

#[derive(Clone, PartialEq, Eq)]
pub struct Draft {
    pub id: DraftId,
    pub revision: u64,
    pub created_at: OffsetDateTime,
    pub flushed_at: OffsetDateTime,
    pub target: DraftTarget,
    pub base: Option<DraftBase>,
    pub current: DraftFields,
}

/// Why a draft file could not be read.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum UnreadableReason {
    Damaged,
    NewerFormat { version: u64 },
    Access,
    NotOwned,
    NotAFile,
}

/// An object whose keys are inserted in order, so it is ordered whether or
/// not `serde_json` keeps insertion order.
fn object(mut entries: Vec<(&str, Value)>) -> Value {
    entries.sort_by(|left, right| left.0.cmp(right.0));
    Value::Object(
        entries
            .into_iter()
            .map(|(key, value)| (key.to_owned(), value))
            .collect::<Map<String, Value>>(),
    )
}

fn time_text(time: OffsetDateTime) -> Value {
    json!(
        time.to_offset(UtcOffset::UTC)
            .format(&Rfc3339)
            .expect("an RFC 3339 time formats")
    )
}

fn fields_value(fields: &DraftFields) -> Value {
    match fields {
        DraftFields::Document { title, body } => object(vec![
            ("kind", json!("document")),
            ("title", json!(title)),
            ("body", json!(body)),
        ]),
        DraftFields::Ticket {
            title,
            ticket_type,
            status,
            project,
            team,
            deps,
            parent,
            body,
        } => object(vec![
            ("kind", json!("ticket")),
            ("title", json!(title)),
            ("type", json!(ticket_type)),
            ("status", json!(status)),
            ("project", json!(project)),
            ("team", json!(team)),
            ("deps", json!(deps)),
            ("parent", json!(parent)),
            ("body", json!(body)),
        ]),
        DraftFields::Comment { body } => {
            object(vec![("kind", json!("comment")), ("body", json!(body))])
        }
    }
}

fn target_value(target: &DraftTarget) -> Value {
    let comment = target.comment.as_ref().map_or(Value::Null, |comment| {
        object(vec![
            ("comment_id", json!(comment.comment_id.to_string())),
            (
                "parent_id",
                json!(comment.parent_id.as_ref().map(ToString::to_string)),
            ),
        ])
    });
    object(vec![
        ("repository", json!(target.repository)),
        (
            "item_kind",
            json!(match target.item_kind {
                ItemKind::Document => "document",
                ItemKind::Ticket => "ticket",
            }),
        ),
        ("item_id", json!(target.item_id.to_string())),
        ("comment", comment),
    ])
}

fn base_value(base: &DraftBase) -> Value {
    object(vec![
        ("observation", json!(base.observation)),
        ("path", json!(base.path)),
        (
            "context",
            object(vec![
                (
                    "kind",
                    json!(match base.context.kind {
                        ContextKind::Primary => "primary",
                        ContextKind::Unverified => "unverified",
                        ContextKind::Active => "active",
                    }),
                ),
                ("branch", json!(base.context.branch)),
                ("worktree", json!(base.context.worktree)),
                ("head_oid", json!(base.context.head_oid)),
            ]),
        ),
        ("source", json!(base.source)),
        ("fields", fields_value(&base.fields)),
    ])
}

/// Encodes a draft as its version 1 file: pretty JSON with every object's
/// keys sorted, and a trailing newline.
#[doc(hidden)]
pub fn encode(draft: &Draft) -> Vec<u8> {
    let value = object(vec![
        ("format", json!(FORMAT)),
        ("version", json!(VERSION)),
        ("id", json!(draft.id.to_string())),
        ("revision", json!(draft.revision)),
        ("created_at", time_text(draft.created_at)),
        ("flushed_at", time_text(draft.flushed_at)),
        ("target", target_value(&draft.target)),
        ("base", draft.base.as_ref().map_or(Value::Null, base_value)),
        ("current", fields_value(&draft.current)),
    ]);
    let mut bytes = serde_json::to_vec_pretty(&value).expect("a JSON value serializes");
    bytes.push(b'\n');
    bytes
}

#[derive(Deserialize)]
struct FileDraft {
    id: String,
    revision: u64,
    created_at: String,
    flushed_at: String,
    target: FileTarget,
    base: Option<FileBase>,
    current: FileFields,
}

#[derive(Deserialize)]
struct FileTarget {
    repository: String,
    item_kind: FileItemKind,
    item_id: String,
    comment: Option<FileComment>,
}

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
enum FileItemKind {
    Document,
    Ticket,
}

#[derive(Deserialize)]
struct FileComment {
    comment_id: String,
    parent_id: Option<String>,
}

#[derive(Deserialize)]
struct FileBase {
    observation: Option<String>,
    path: String,
    context: FileContext,
    source: Option<String>,
    fields: FileFields,
}

#[derive(Deserialize)]
struct FileContext {
    kind: FileContextKind,
    branch: Option<String>,
    worktree: String,
    head_oid: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
enum FileContextKind {
    Primary,
    Unverified,
    Active,
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
enum FileFields {
    Document {
        title: String,
        body: String,
    },
    Ticket {
        title: String,
        #[serde(rename = "type")]
        ticket_type: String,
        status: String,
        project: Option<String>,
        team: Option<String>,
        deps: Vec<String>,
        parent: Option<String>,
        body: String,
    },
    Comment {
        body: String,
    },
}

fn item_id(text: &str) -> Result<ItemId, UnreadableReason> {
    text.parse().map_err(|_| UnreadableReason::Damaged)
}

fn time(text: &str) -> Result<OffsetDateTime, UnreadableReason> {
    OffsetDateTime::parse(text, &Rfc3339)
        .map(|time| time.to_offset(UtcOffset::UTC))
        .map_err(|_| UnreadableReason::Damaged)
}

impl From<FileFields> for DraftFields {
    fn from(fields: FileFields) -> Self {
        match fields {
            FileFields::Document { title, body } => DraftFields::Document { title, body },
            FileFields::Ticket {
                title,
                ticket_type,
                status,
                project,
                team,
                deps,
                parent,
                body,
            } => DraftFields::Ticket {
                title,
                ticket_type,
                status,
                project,
                team,
                deps,
                parent,
                body,
            },
            FileFields::Comment { body } => DraftFields::Comment { body },
        }
    }
}

fn draft_from_file(file: FileDraft) -> Result<Draft, UnreadableReason> {
    let comment = match file.target.comment {
        None => None,
        Some(comment) => Some(CommentTarget {
            comment_id: item_id(&comment.comment_id)?,
            parent_id: comment.parent_id.as_deref().map(item_id).transpose()?,
        }),
    };
    let base = file.base.map(|base| DraftBase {
        observation: base.observation,
        path: base.path,
        context: BaseContext {
            kind: match base.context.kind {
                FileContextKind::Primary => ContextKind::Primary,
                FileContextKind::Unverified => ContextKind::Unverified,
                FileContextKind::Active => ContextKind::Active,
            },
            branch: base.context.branch,
            worktree: base.context.worktree,
            head_oid: base.context.head_oid,
        },
        source: base.source,
        fields: base.fields.into(),
    });
    Ok(Draft {
        id: file.id.parse().map_err(|_| UnreadableReason::Damaged)?,
        revision: file.revision,
        created_at: time(&file.created_at)?,
        flushed_at: time(&file.flushed_at)?,
        target: DraftTarget {
            repository: file.target.repository,
            item_kind: match file.target.item_kind {
                FileItemKind::Document => ItemKind::Document,
                FileItemKind::Ticket => ItemKind::Ticket,
            },
            item_id: item_id(&file.target.item_id)?,
            comment,
        },
        base,
        current: file.current.into(),
    })
}

/// Decodes a draft file. It reads `format` and `version` first; a version
/// above 1 is `NewerFormat`, and anything else that cannot be read in full
/// is `Damaged`. It never touches the file.
#[doc(hidden)]
pub fn decode(bytes: &[u8]) -> Result<Draft, UnreadableReason> {
    let value: Value = serde_json::from_slice(bytes).map_err(|_| UnreadableReason::Damaged)?;
    if value.get("format").and_then(Value::as_str) != Some(FORMAT) {
        return Err(UnreadableReason::Damaged);
    }
    let version = value
        .get("version")
        .and_then(Value::as_u64)
        .ok_or(UnreadableReason::Damaged)?;
    if version > VERSION {
        return Err(UnreadableReason::NewerFormat { version });
    }
    if version != VERSION {
        return Err(UnreadableReason::Damaged);
    }
    let file: FileDraft = serde_json::from_value(value).map_err(|_| UnreadableReason::Damaged)?;
    draft_from_file(file)
}
