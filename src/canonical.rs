//! Version 1 canonical configuration and Markdown domain contracts.

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
    path::{Component, Path, PathBuf},
    str::FromStr,
};

use serde_yaml::{Mapping, Value};
use time::{OffsetDateTime, UtcOffset, format_description::well_known::Rfc3339};

pub const CONFIG_PATH: &str = ".manyhands/config.toml";

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ItemId(ulid::Ulid);

impl ItemId {
    pub fn generate() -> Self {
        Self(ulid::Ulid::new())
    }
}

impl fmt::Display for ItemId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

impl FromStr for ItemId {
    type Err = ValidationProblem;

    fn from_str(source: &str) -> Result<Self, Self::Err> {
        const CROCKFORD_BASE32: &str = "0123456789ABCDEFGHJKMNPQRSTVWXYZ";

        if source.len() != 26
            || !source
                .chars()
                .all(|character| CROCKFORD_BASE32.contains(character))
        {
            return Err(validation_problem(
                ValidationCode::InvalidField,
                "item ID must be a 26-character uppercase Crockford Base32 ULID",
            ));
        }

        source.parse().map(Self).map_err(|_| {
            validation_problem(ValidationCode::InvalidField, "item ID must be a valid ULID")
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ItemKind {
    Document,
    Ticket,
    Comment,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ValidationCode {
    InvalidPath,
    MissingFrontMatter,
    MalformedFrontMatter,
    MalformedConfiguration,
    MissingField,
    InvalidField,
    KindPathMismatch,
    DuplicateId,
    MissingCommentItem,
    MissingParent,
    CrossItemParent,
    CommentCycle,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidationProblem {
    pub path: PathBuf,
    pub code: ValidationCode,
    pub message: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RepositoryConfig {
    pub primary_branch: String,
    pub publication_remote: Option<String>,
    pub unknown: toml::Table,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Document {
    pub id: ItemId,
    pub title: String,
    pub body: String,
    pub unknown: Mapping,
}

impl Document {
    pub fn set_title(&mut self, title: String) -> Result<(), ValidationProblem> {
        validate_non_empty_item_text(&title, "title")?;
        self.title = title;
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Ticket {
    pub id: ItemId,
    pub title: String,
    pub ticket_type: String,
    pub status: String,
    pub project: Option<String>,
    pub team: Option<String>,
    pub closed_at: Option<OffsetDateTime>,
    pub closed_by: Option<String>,
    pub body: String,
    pub unknown: Mapping,
}

/// The optional ticket front matter keys that relate one ticket to others
/// and give it a short code. They are not fields of `Ticket`: they stay in
/// `Ticket::unknown`, where the serializer keeps them as it keeps any key it
/// does not define, and `ticket_relationships` reads them from there.
pub const RELATIONSHIP_KEYS: [&str; 3] = ["slug", "parent", "deps"];

/// Why a relationship value was ignored.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RelationshipProblemCode {
    /// `deps` is not a sequence, or a `deps` entry or `parent` is not a
    /// string.
    WrongType,
    /// A `deps` entry or `parent` is a string that is not an item ID.
    InvalidId,
    /// A `deps` entry or `parent` names the ticket itself.
    SelfReference,
    /// A `deps` entry repeats an earlier one.
    DuplicateDependency,
    /// `slug` is not a string that follows the short code grammar.
    InvalidSlug,
}

impl RelationshipProblemCode {
    pub const ALL: [Self; 5] = [
        Self::WrongType,
        Self::InvalidId,
        Self::SelfReference,
        Self::DuplicateDependency,
        Self::InvalidSlug,
    ];
}

/// One ignored relationship value. `detail` is the item ID the problem is
/// about, when the value was one; it is never text taken from the file.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RelationshipProblem {
    pub code: RelationshipProblemCode,
    pub detail: Option<ItemId>,
}

/// A ticket's `slug`, `parent` and `deps`, as far as they are valid.
///
/// `deps` is in the file's order, each ID once. A value that is not valid
/// is left out of its field and listed in `problems`, in the order slug,
/// parent, then each `deps` entry; none of them makes the ticket
/// nonconforming. Whether a target exists, and whether it is a ticket,
/// cannot be known from one file and is not checked here.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TicketRelationships {
    pub slug: Option<String>,
    pub parent: Option<ItemId>,
    pub deps: Vec<ItemId>,
    pub problems: Vec<RelationshipProblem>,
}

/// Reads the relationship keys from `ticket.unknown`, which is left as it
/// is.
///
/// A key whose value is YAML null is read as an absent key. A short code
/// written with uppercase letters is read in lowercase; the file keeps its
/// own spelling.
///
/// A short code must be a YAML string. One a plain scalar would read as
/// another type has to be quoted when it is written: `1e-12345` unquoted is
/// a number, and is reported as an invalid slug.
pub fn ticket_relationships(ticket: &Ticket) -> TicketRelationships {
    let mut view = TicketRelationships::default();
    let mut ignore = |code, detail| view.problems.push(RelationshipProblem { code, detail });
    let value = |key: &str| ticket.unknown.get(key).filter(|value| !value.is_null());

    let slug = value("slug").and_then(|value| {
        let slug = value.as_str().and_then(normalized_slug);
        if slug.is_none() {
            ignore(RelationshipProblemCode::InvalidSlug, None);
        }
        slug
    });
    let parent = value("parent").and_then(|value| match relationship_target(&ticket.id, value) {
        Ok(parent) => Some(parent),
        Err((code, detail)) => {
            ignore(code, detail);
            None
        }
    });
    let mut deps = Vec::new();
    match value("deps") {
        None => {}
        Some(Value::Sequence(entries)) => {
            for entry in entries {
                match relationship_target(&ticket.id, entry) {
                    Ok(dependency) if deps.contains(&dependency) => ignore(
                        RelationshipProblemCode::DuplicateDependency,
                        Some(dependency),
                    ),
                    Ok(dependency) => deps.push(dependency),
                    Err((code, detail)) => ignore(code, detail),
                }
            }
        }
        Some(_) => ignore(RelationshipProblemCode::WrongType, None),
    }

    view.slug = slug;
    view.parent = parent;
    view.deps = deps;
    view
}

/// The other ticket a `parent` value or a `deps` entry names, or why it
/// names none.
fn relationship_target(
    ticket: &ItemId,
    value: &Value,
) -> Result<ItemId, (RelationshipProblemCode, Option<ItemId>)> {
    let text = value
        .as_str()
        .ok_or((RelationshipProblemCode::WrongType, None))?;
    let target: ItemId = text
        .parse()
        .map_err(|_| (RelationshipProblemCode::InvalidId, None))?;
    if target == *ticket {
        return Err((RelationshipProblemCode::SelfReference, Some(target)));
    }
    Ok(target)
}

/// Whether `value` is a short code: an optional prefix of one to eight
/// lowercase letters or digits, initials of two or three, and a code of
/// five to eight lowercase Crockford Base32 characters, joined by hyphens.
///
/// Only the lowercase spelling is one: it is how a short code is stored
/// and displayed. `normalized_slug` gives it for a code written in any
/// case.
pub fn is_valid_slug(value: &str) -> bool {
    const CROCKFORD_BASE32_LOWERCASE: &[u8] = b"0123456789abcdefghjkmnpqrstvwxyz";

    let alphanumeric = |part: &str, lengths: std::ops::RangeInclusive<usize>| {
        lengths.contains(&part.len())
            && part
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
    };
    let parts: Vec<&str> = value.split('-').collect();
    let (prefix, initials, code) = match parts.as_slice() {
        [initials, code] => (None, initials, code),
        [prefix, initials, code] => (Some(prefix), initials, code),
        _ => return false,
    };
    prefix.is_none_or(|prefix| alphanumeric(prefix, 1..=8))
        && alphanumeric(initials, 2..=3)
        && (5..=8).contains(&code.len())
        && code
            .bytes()
            .all(|byte| CROCKFORD_BASE32_LOWERCASE.contains(&byte))
}

/// `value` in lowercase when that is a short code, so that a code is
/// matched without regard to the case it was written in. Only ASCII
/// letters change case; everything else about the grammar is as strict as
/// `is_valid_slug`.
pub fn normalized_slug(value: &str) -> Option<String> {
    let slug = value.to_ascii_lowercase();
    is_valid_slug(&slug).then_some(slug)
}

#[derive(Clone, Debug, PartialEq)]
pub struct Comment {
    pub id: ItemId,
    pub item_id: ItemId,
    pub parent_id: Option<ItemId>,
    pub created_at: OffsetDateTime,
    pub body: String,
    pub unknown: Mapping,
}

#[derive(Clone, Debug, PartialEq)]
pub enum CanonicalItem {
    Document(Document),
    Ticket(Ticket),
    Comment(Comment),
}

#[derive(Clone, Debug, PartialEq)]
pub struct ValidatedContext {
    pub items: Vec<CanonicalItem>,
    pub problems: Vec<ValidationProblem>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CommentThread {
    pub comment: Comment,
    pub replies: Vec<CommentThread>,
}

impl CanonicalItem {
    pub fn body(&self) -> &str {
        match self {
            Self::Document(document) => &document.body,
            Self::Ticket(ticket) => &ticket.body,
            Self::Comment(comment) => &comment.body,
        }
    }
}

pub fn parse_item(path: &Path, source: &str) -> Result<CanonicalItem, ValidationProblem> {
    let canonical_path = classify_canonical_path(path)?;
    let (front_matter, body) = split_front_matter(path, source)?;
    let value: Value = serde_yaml::from_str(front_matter).map_err(|error| {
        item_problem(
            path,
            ValidationCode::MalformedFrontMatter,
            format!("invalid YAML: {error}"),
        )
    })?;
    let mut values = value.as_mapping().cloned().ok_or_else(|| {
        item_problem(
            path,
            ValidationCode::MalformedFrontMatter,
            "front matter must be a YAML mapping",
        )
    })?;

    match required_bool(path, &mut values, "manyhands_managed")? {
        true => {}
        false => {
            return Err(item_problem(
                path,
                ValidationCode::InvalidField,
                "manyhands_managed must be true",
            ));
        }
    }
    let kind = required_item_string(path, &mut values, "manyhands_kind")?;
    let item = match kind.as_str() {
        "document" => CanonicalItem::Document(Document {
            id: required_item_id(path, &mut values, "id")?,
            title: required_item_string(path, &mut values, "title")?,
            body: body.to_owned(),
            unknown: values,
        }),
        "ticket" => {
            let closed_at = optional_time(path, &mut values, "closed_at")?;
            let closed_by = optional_item_string(path, &mut values, "closed_by")?;
            if closed_at.is_some() != closed_by.is_some() {
                return Err(item_problem(
                    path,
                    ValidationCode::InvalidField,
                    "closed_at and closed_by must either both be present or both be absent",
                ));
            }
            CanonicalItem::Ticket(Ticket {
                id: required_item_id(path, &mut values, "id")?,
                title: required_item_string(path, &mut values, "title")?,
                ticket_type: required_item_string(path, &mut values, "type")?,
                status: required_item_string(path, &mut values, "status")?,
                project: optional_item_string(path, &mut values, "project")?,
                team: optional_item_string(path, &mut values, "team")?,
                closed_at,
                closed_by,
                body: body.to_owned(),
                unknown: values,
            })
        }
        "comment" => CanonicalItem::Comment(Comment {
            id: required_item_id(path, &mut values, "id")?,
            item_id: required_item_id(path, &mut values, "item_id")?,
            parent_id: optional_item_id(path, &mut values, "parent_id")?,
            created_at: required_time(path, &mut values, "created_at")?,
            body: body.to_owned(),
            unknown: values,
        }),
        _ => Err(item_problem(
            path,
            ValidationCode::InvalidField,
            "manyhands_kind must be document, ticket, or comment",
        ))?,
    };
    validate_item_path(path, &canonical_path, &item)?;
    Ok(item)
}

pub fn validate_context(sources: impl IntoIterator<Item = (PathBuf, String)>) -> ValidatedContext {
    let mut problems = Vec::new();
    let mut candidates = Vec::new();
    for (path, source) in sources {
        match parse_item(&path, &source) {
            Ok(item) => candidates.push((path, item)),
            Err(problem) => problems.push(problem),
        }
    }

    let mut ids = BTreeMap::<ItemId, Vec<usize>>::new();
    for (index, (_, item)) in candidates.iter().enumerate() {
        ids.entry(item_id(item).clone()).or_default().push(index);
    }
    let mut duplicates = BTreeSet::new();
    for (id, indexes) in &ids {
        if indexes.len() > 1 {
            duplicates.insert(id.clone());
            for &index in indexes {
                problems.push(item_problem(
                    &candidates[index].0,
                    ValidationCode::DuplicateId,
                    "item ID is duplicated in this context",
                ));
            }
        }
    }

    let documents_and_tickets = candidates
        .iter()
        .filter_map(|(_, item)| match item {
            CanonicalItem::Document(document) => Some(document.id.clone()),
            CanonicalItem::Ticket(ticket) => Some(ticket.id.clone()),
            CanonicalItem::Comment(_) => None,
        })
        .filter(|id| !duplicates.contains(id))
        .collect::<BTreeSet<_>>();
    let comments = candidates
        .iter()
        .enumerate()
        .filter_map(|(index, (path, item))| match item {
            CanonicalItem::Comment(comment) if !duplicates.contains(&comment.id) => {
                Some((index, path, comment))
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    let comment_indexes = comments
        .iter()
        .map(|(index, _, comment)| (comment.id.clone(), *index))
        .collect::<BTreeMap<_, _>>();
    let comment_items = comments
        .iter()
        .map(|(_, _, comment)| (comment.id.clone(), comment.item_id.clone()))
        .collect::<BTreeMap<_, _>>();
    let mut invalid_comments = BTreeSet::new();

    for (_, path, comment) in &comments {
        if !documents_and_tickets.contains(&comment.item_id) {
            invalid_comments.insert(comment.id.clone());
            problems.push(item_problem(
                path,
                ValidationCode::MissingCommentItem,
                "comment item_id does not refer to a document or ticket",
            ));
        }
        if let Some(parent_id) = &comment.parent_id {
            match comment_items.get(parent_id) {
                None => {
                    invalid_comments.insert(comment.id.clone());
                    problems.push(item_problem(
                        path,
                        ValidationCode::MissingParent,
                        "comment parent_id does not refer to a comment",
                    ));
                }
                Some(parent_item_id) if parent_item_id != &comment.item_id => {
                    invalid_comments.insert(comment.id.clone());
                    problems.push(item_problem(
                        path,
                        ValidationCode::CrossItemParent,
                        "comment parent_id belongs to a different item",
                    ));
                }
                Some(_) => {}
            }
        }
    }

    let mut cycle_detector = CommentCycleDetector {
        comment_indexes: &comment_indexes,
        comment_items: &comment_items,
        candidates: &candidates,
        visiting: BTreeMap::new(),
        visited: BTreeSet::new(),
        stack: Vec::new(),
        cycles: BTreeSet::new(),
    };
    for (_, _, comment) in &comments {
        cycle_detector.visit(&comment.id);
    }
    for (_, path, comment) in &comments {
        if cycle_detector.cycles.contains(&comment.id) {
            invalid_comments.insert(comment.id.clone());
            problems.push(item_problem(
                path,
                ValidationCode::CommentCycle,
                "comment parent relationships contain a cycle",
            ));
        }
    }

    loop {
        let newly_invalid = comments
            .iter()
            .filter_map(|(_, path, comment)| {
                let parent_id = comment.parent_id.as_ref()?;
                (!invalid_comments.contains(&comment.id) && invalid_comments.contains(parent_id))
                    .then_some((path, comment))
            })
            .collect::<Vec<_>>();
        if newly_invalid.is_empty() {
            break;
        }
        for (path, comment) in newly_invalid {
            invalid_comments.insert(comment.id.clone());
            problems.push(item_problem(
                path,
                ValidationCode::MissingParent,
                "comment parent_id refers to a nonconforming or unavailable comment",
            ));
        }
    }

    let items = candidates
        .into_iter()
        .filter_map(|(_, item)| match &item {
            CanonicalItem::Document(document) if !duplicates.contains(&document.id) => Some(item),
            CanonicalItem::Ticket(ticket) if !duplicates.contains(&ticket.id) => Some(item),
            CanonicalItem::Comment(comment)
                if !duplicates.contains(&comment.id) && !invalid_comments.contains(&comment.id) =>
            {
                Some(item)
            }
            _ => None,
        })
        .collect();
    ValidatedContext { items, problems }
}

pub fn ordered_comment_threads(context: &ValidatedContext) -> Vec<CommentThread> {
    let mut children = BTreeMap::<Option<ItemId>, Vec<Comment>>::new();
    for item in &context.items {
        if let CanonicalItem::Comment(comment) = item {
            children
                .entry(comment.parent_id.clone())
                .or_default()
                .push(comment.clone());
        }
    }
    build_comment_threads(None, &mut children)
}

fn item_id(item: &CanonicalItem) -> &ItemId {
    match item {
        CanonicalItem::Document(document) => &document.id,
        CanonicalItem::Ticket(ticket) => &ticket.id,
        CanonicalItem::Comment(comment) => &comment.id,
    }
}

struct CommentCycleDetector<'a> {
    comment_indexes: &'a BTreeMap<ItemId, usize>,
    comment_items: &'a BTreeMap<ItemId, ItemId>,
    candidates: &'a [(PathBuf, CanonicalItem)],
    visiting: BTreeMap<ItemId, usize>,
    visited: BTreeSet<ItemId>,
    stack: Vec<ItemId>,
    cycles: BTreeSet<ItemId>,
}

impl CommentCycleDetector<'_> {
    fn visit(&mut self, id: &ItemId) {
        if self.visited.contains(id) {
            return;
        }
        if let Some(&start) = self.visiting.get(id) {
            self.cycles.extend(self.stack[start..].iter().cloned());
            return;
        }
        let Some(&index) = self.comment_indexes.get(id) else {
            return;
        };
        let CanonicalItem::Comment(comment) = &self.candidates[index].1 else {
            return;
        };
        let parent_id = comment.parent_id.clone();
        let item_id = comment.item_id.clone();
        self.visiting.insert(id.clone(), self.stack.len());
        self.stack.push(id.clone());
        if let Some(parent_id) = parent_id
            && self.comment_items.get(&parent_id) == Some(&item_id)
        {
            self.visit(&parent_id);
        }
        self.stack.pop();
        self.visiting.remove(id);
        self.visited.insert(id.clone());
    }
}

fn build_comment_threads(
    parent_id: Option<ItemId>,
    children: &mut BTreeMap<Option<ItemId>, Vec<Comment>>,
) -> Vec<CommentThread> {
    let mut comments = children.remove(&parent_id).unwrap_or_default();
    comments.sort_by(|left, right| {
        left.created_at
            .cmp(&right.created_at)
            .then_with(|| left.id.cmp(&right.id))
    });
    comments
        .into_iter()
        .map(|comment| CommentThread {
            replies: build_comment_threads(Some(comment.id.clone()), children),
            comment,
        })
        .collect()
}

enum CanonicalPath {
    Document,
    Ticket(ItemId),
    Comment { item_id: ItemId, id: ItemId },
}

fn classify_canonical_path(path: &Path) -> Result<CanonicalPath, ValidationProblem> {
    let text = path.to_str().ok_or_else(|| {
        item_problem(
            path,
            ValidationCode::InvalidPath,
            "path components must be valid UTF-8",
        )
    })?;
    if text.contains('\\') {
        return Err(item_problem(
            path,
            ValidationCode::InvalidPath,
            "path must use forward slashes",
        ));
    }
    if text.is_empty() || has_invalid_raw_path_segments(text) {
        return Err(item_problem(
            path,
            ValidationCode::InvalidPath,
            "path must not be empty or contain empty, . or .. components",
        ));
    }

    let mut components = Vec::new();
    for component in path.components() {
        match component {
            Component::Normal(component) => {
                components.push(component.to_str().ok_or_else(|| {
                    item_problem(
                        path,
                        ValidationCode::InvalidPath,
                        "path components must be valid UTF-8",
                    )
                })?)
            }
            Component::CurDir
            | Component::ParentDir
            | Component::RootDir
            | Component::Prefix(_) => {
                return Err(item_problem(
                    path,
                    ValidationCode::InvalidPath,
                    "path must be relative and contain only normal components",
                ));
            }
        }
    }

    if components.starts_with(&[".manyhands", "worktrees"]) {
        return Err(item_problem(
            path,
            ValidationCode::InvalidPath,
            "paths under .manyhands/worktrees are not canonical items",
        ));
    }
    match components.as_slice() {
        ["docs", rest @ ..]
            if !rest.is_empty() && rest.last().is_some_and(|name| name.ends_with(".md")) =>
        {
            Ok(CanonicalPath::Document)
        }
        [".manyhands", "tickets", id, "ticket.md"] => {
            Ok(CanonicalPath::Ticket(path_item_id(path, id)?))
        }
        [".manyhands", "comments", item_id, filename] => {
            let id = filename.strip_suffix(".md").ok_or_else(|| {
                item_problem(
                    path,
                    ValidationCode::InvalidPath,
                    "comment filename must be an item ID followed by .md",
                )
            })?;
            Ok(CanonicalPath::Comment {
                item_id: path_item_id(path, item_id)?,
                id: path_item_id(path, id)?,
            })
        }
        _ => Err(item_problem(
            path,
            ValidationCode::InvalidPath,
            "path is not a canonical document, ticket, or comment location",
        )),
    }
}

/// The kind of item a repository-relative path can hold, by the path alone:
/// `docs/**/*.md`, `.manyhands/tickets/<id>/ticket.md` or
/// `.manyhands/comments/<item id>/<id>.md`. Any other path, and any path
/// that is absolute, empty or has a `.`, `..` or empty component, is
/// `InvalidPath`. Nothing is read.
pub fn item_path_kind(path: &Path) -> Result<ItemKind, ValidationProblem> {
    Ok(match classify_canonical_path(path)? {
        CanonicalPath::Document => ItemKind::Document,
        CanonicalPath::Ticket(_) => ItemKind::Ticket,
        CanonicalPath::Comment { .. } => ItemKind::Comment,
    })
}

fn has_invalid_raw_path_segments(path: &str) -> bool {
    path.split('/')
        .any(|segment| segment.is_empty() || matches!(segment, "." | ".."))
}

fn path_item_id(path: &Path, value: &str) -> Result<ItemId, ValidationProblem> {
    value.parse().map_err(|_| {
        item_problem(
            path,
            ValidationCode::InvalidPath,
            "path item ID must be a canonical ULID",
        )
    })
}

fn validate_item_path(
    path: &Path,
    canonical_path: &CanonicalPath,
    item: &CanonicalItem,
) -> Result<(), ValidationProblem> {
    match (canonical_path, item) {
        (CanonicalPath::Document, CanonicalItem::Document(_)) => Ok(()),
        (CanonicalPath::Ticket(path_id), CanonicalItem::Ticket(ticket))
            if path_id == &ticket.id =>
        {
            Ok(())
        }
        (
            CanonicalPath::Comment {
                item_id: path_item_id,
                id: path_id,
            },
            CanonicalItem::Comment(comment),
        ) if path_item_id == &comment.item_id && path_id == &comment.id => Ok(()),
        (CanonicalPath::Ticket(_), CanonicalItem::Ticket(_))
        | (CanonicalPath::Comment { .. }, CanonicalItem::Comment(_)) => Err(item_problem(
            path,
            ValidationCode::InvalidField,
            "path item ID does not match front matter",
        )),
        _ => Err(item_problem(
            path,
            ValidationCode::KindPathMismatch,
            "item kind does not match its canonical path",
        )),
    }
}

pub fn serialize_item(item: &CanonicalItem) -> Result<String, ValidationProblem> {
    let (mut values, body) = match item {
        CanonicalItem::Document(document) => {
            validate_non_empty_item_text(&document.title, "title")?;
            let mut values = document.unknown.clone();
            insert_known(&mut values, "id", document.id.to_string());
            insert_known(&mut values, "title", document.title.clone());
            (values, &document.body)
        }
        CanonicalItem::Ticket(ticket) => {
            validate_non_empty_item_text(&ticket.title, "title")?;
            validate_non_empty_item_text(&ticket.ticket_type, "type")?;
            validate_non_empty_item_text(&ticket.status, "status")?;
            validate_optional_text(&ticket.project, "project")?;
            validate_optional_text(&ticket.team, "team")?;
            validate_optional_text(&ticket.closed_by, "closed_by")?;
            if ticket.closed_at.is_some() != ticket.closed_by.is_some() {
                return Err(pathless_item_problem(
                    ValidationCode::InvalidField,
                    "closed_at and closed_by must either both be present or both be absent",
                ));
            }
            let mut values = ticket.unknown.clone();
            insert_known(&mut values, "id", ticket.id.to_string());
            insert_known(&mut values, "title", ticket.title.clone());
            insert_known(&mut values, "type", ticket.ticket_type.clone());
            insert_known(&mut values, "status", ticket.status.clone());
            insert_optional_known(&mut values, "project", &ticket.project);
            insert_optional_known(&mut values, "team", &ticket.team);
            remove_known(&mut values, "closed_at");
            if let Some(closed_at) = ticket.closed_at {
                insert_known(&mut values, "closed_at", format_time(closed_at)?);
            }
            insert_optional_known(&mut values, "closed_by", &ticket.closed_by);
            (values, &ticket.body)
        }
        CanonicalItem::Comment(comment) => {
            let mut values = comment.unknown.clone();
            insert_known(&mut values, "id", comment.id.to_string());
            insert_known(&mut values, "item_id", comment.item_id.to_string());
            remove_known(&mut values, "parent_id");
            if let Some(parent_id) = &comment.parent_id {
                insert_known(&mut values, "parent_id", parent_id.to_string());
            }
            insert_known(&mut values, "created_at", format_time(comment.created_at)?);
            (values, &comment.body)
        }
    };
    insert_known(&mut values, "manyhands_managed", true);
    let kind = match item {
        CanonicalItem::Document(_) => "document",
        CanonicalItem::Ticket(_) => "ticket",
        CanonicalItem::Comment(_) => "comment",
    };
    insert_known(&mut values, "manyhands_kind", kind);

    let yaml = serde_yaml::to_string(&values).map_err(|error| {
        pathless_item_problem(
            ValidationCode::InvalidField,
            format!("cannot serialize YAML front matter: {error}"),
        )
    })?;
    let yaml = yaml.strip_prefix("---\n").unwrap_or(&yaml);
    Ok(format!("---\n{yaml}---\n{body}"))
}

fn split_front_matter<'a>(
    path: &Path,
    source: &'a str,
) -> Result<(&'a str, &'a str), ValidationProblem> {
    let content_start = if source.starts_with("---\n") {
        4
    } else if source.starts_with("---\r\n") {
        5
    } else {
        return Err(item_problem(
            path,
            ValidationCode::MissingFrontMatter,
            "front matter must begin at byte zero with ---",
        ));
    };
    let mut line_start = content_start;
    while line_start < source.len() {
        let remaining = &source[line_start..];
        let Some(newline) = remaining.find('\n') else {
            if remaining == "---" {
                return Ok((&source[content_start..line_start], ""));
            }
            break;
        };
        let line_end = line_start + newline;
        let line = source[line_start..line_end]
            .strip_suffix('\r')
            .unwrap_or(&source[line_start..line_end]);
        if line == "---" {
            return Ok((&source[content_start..line_start], &source[line_end + 1..]));
        }
        line_start = line_end + 1;
    }
    Err(item_problem(
        path,
        ValidationCode::MalformedFrontMatter,
        "front matter is missing a closing --- delimiter",
    ))
}

fn required_bool(path: &Path, values: &mut Mapping, name: &str) -> Result<bool, ValidationProblem> {
    let value = remove_known(values, name).ok_or_else(|| {
        item_problem(
            path,
            ValidationCode::MissingField,
            format!("missing {name}"),
        )
    })?;
    value.as_bool().ok_or_else(|| {
        item_problem(
            path,
            ValidationCode::InvalidField,
            format!("{name} must be a boolean"),
        )
    })
}

fn required_item_string(
    path: &Path,
    values: &mut Mapping,
    name: &str,
) -> Result<String, ValidationProblem> {
    let value = remove_known(values, name).ok_or_else(|| {
        item_problem(
            path,
            ValidationCode::MissingField,
            format!("missing {name}"),
        )
    })?;
    let value = value.as_str().ok_or_else(|| {
        item_problem(
            path,
            ValidationCode::InvalidField,
            format!("{name} must be a string"),
        )
    })?;
    validate_item_text(path, value, name)?;
    Ok(value.to_owned())
}

fn optional_item_string(
    path: &Path,
    values: &mut Mapping,
    name: &str,
) -> Result<Option<String>, ValidationProblem> {
    let Some(value) = remove_known(values, name) else {
        return Ok(None);
    };
    let value = value.as_str().ok_or_else(|| {
        item_problem(
            path,
            ValidationCode::InvalidField,
            format!("{name} must be a string"),
        )
    })?;
    validate_item_text(path, value, name)?;
    Ok(Some(value.to_owned()))
}

fn required_item_id(
    path: &Path,
    values: &mut Mapping,
    name: &str,
) -> Result<ItemId, ValidationProblem> {
    let value = required_item_string(path, values, name)?;
    value
        .parse()
        .map_err(|error: ValidationProblem| ValidationProblem {
            path: path.to_owned(),
            ..error
        })
}

fn optional_item_id(
    path: &Path,
    values: &mut Mapping,
    name: &str,
) -> Result<Option<ItemId>, ValidationProblem> {
    let Some(value) = optional_item_string(path, values, name)? else {
        return Ok(None);
    };
    value
        .parse()
        .map(Some)
        .map_err(|error: ValidationProblem| ValidationProblem {
            path: path.to_owned(),
            ..error
        })
}

fn required_time(
    path: &Path,
    values: &mut Mapping,
    name: &str,
) -> Result<OffsetDateTime, ValidationProblem> {
    let value = required_item_string(path, values, name)?;
    parse_time(path, &value, name)
}

fn optional_time(
    path: &Path,
    values: &mut Mapping,
    name: &str,
) -> Result<Option<OffsetDateTime>, ValidationProblem> {
    let Some(value) = optional_item_string(path, values, name)? else {
        return Ok(None);
    };
    parse_time(path, &value, name).map(Some)
}

fn parse_time(path: &Path, value: &str, name: &str) -> Result<OffsetDateTime, ValidationProblem> {
    let time = OffsetDateTime::parse(value, &Rfc3339).map_err(|_| {
        item_problem(
            path,
            ValidationCode::InvalidField,
            format!("{name} must be RFC3339"),
        )
    })?;
    if time.offset() != UtcOffset::UTC {
        return Err(item_problem(
            path,
            ValidationCode::InvalidField,
            format!("{name} must use UTC"),
        ));
    }
    Ok(time)
}

fn format_time(value: OffsetDateTime) -> Result<String, ValidationProblem> {
    if value.offset() != UtcOffset::UTC {
        return Err(pathless_item_problem(
            ValidationCode::InvalidField,
            "timestamps must use UTC",
        ));
    }
    value.format(&Rfc3339).map_err(|error| {
        pathless_item_problem(
            ValidationCode::InvalidField,
            format!("cannot serialize timestamp: {error}"),
        )
    })
}

fn validate_item_text(path: &Path, value: &str, name: &str) -> Result<(), ValidationProblem> {
    if value.is_empty() || value.contains('\0') {
        return Err(item_problem(
            path,
            ValidationCode::InvalidField,
            format!("{name} must be non-empty and contain no NUL characters"),
        ));
    }
    Ok(())
}

fn validate_optional_text(value: &Option<String>, name: &str) -> Result<(), ValidationProblem> {
    if let Some(value) = value {
        validate_non_empty_item_text(value, name)?;
    }
    Ok(())
}

fn remove_known(values: &mut Mapping, name: &str) -> Option<Value> {
    values.remove(Value::String(name.to_owned()))
}

fn insert_known(values: &mut Mapping, name: &str, value: impl Into<Value>) {
    values.insert(Value::String(name.to_owned()), value.into());
}

fn insert_optional_known(values: &mut Mapping, name: &str, value: &Option<String>) {
    remove_known(values, name);
    if let Some(value) = value {
        insert_known(values, name, value.clone());
    }
}

fn item_problem(
    path: &Path,
    code: ValidationCode,
    message: impl Into<String>,
) -> ValidationProblem {
    ValidationProblem {
        path: path.to_owned(),
        code,
        message: message.into(),
    }
}

fn pathless_item_problem(code: ValidationCode, message: impl Into<String>) -> ValidationProblem {
    ValidationProblem {
        path: PathBuf::new(),
        code,
        message: message.into(),
    }
}

fn validate_non_empty_item_text(value: &str, name: &str) -> Result<(), ValidationProblem> {
    if value.is_empty() || value.contains('\0') {
        return Err(pathless_item_problem(
            ValidationCode::InvalidField,
            format!("{name} must be non-empty and contain no NUL characters"),
        ));
    }
    Ok(())
}

pub fn parse_repository_config(source: &str) -> Result<RepositoryConfig, ValidationProblem> {
    let mut values: toml::Table = source.parse().map_err(|error| {
        validation_problem(
            ValidationCode::MalformedConfiguration,
            format!("invalid TOML configuration: {error}"),
        )
    })?;

    let format_version = values.remove("format_version").ok_or_else(|| {
        validation_problem(ValidationCode::MissingField, "missing format_version")
    })?;
    match format_version {
        toml::Value::Integer(1) => {}
        toml::Value::Integer(_) => {
            return Err(validation_problem(
                ValidationCode::InvalidField,
                "unsupported format_version; expected 1",
            ));
        }
        _ => {
            return Err(validation_problem(
                ValidationCode::InvalidField,
                "format_version must be an integer",
            ));
        }
    }

    let primary_branch = required_string(&mut values, "primary_branch")?;
    let publication_remote = optional_string(&mut values, "publication_remote")?;
    validate_primary_branch_name(&primary_branch)?;
    if let Some(publication_remote) = &publication_remote {
        validate_git_short_name(publication_remote, "publication_remote")?;
    }

    Ok(RepositoryConfig {
        primary_branch,
        publication_remote,
        unknown: values,
    })
}

pub fn serialize_repository_config(config: &RepositoryConfig) -> Result<String, ValidationProblem> {
    validate_primary_branch_name(&config.primary_branch)?;
    if let Some(publication_remote) = &config.publication_remote {
        validate_git_short_name(publication_remote, "publication_remote")?;
    }

    let mut values = config.unknown.clone();
    values.remove("format_version");
    values.remove("primary_branch");
    values.remove("publication_remote");
    values.insert("format_version".into(), toml::Value::Integer(1));
    values.insert(
        "primary_branch".into(),
        toml::Value::String(config.primary_branch.clone()),
    );
    if let Some(publication_remote) = &config.publication_remote {
        values.insert(
            "publication_remote".into(),
            toml::Value::String(publication_remote.clone()),
        );
    }

    toml::to_string(&values).map_err(|error| {
        validation_problem(
            ValidationCode::InvalidField,
            format!("cannot serialize TOML configuration: {error}"),
        )
    })
}

fn required_string(values: &mut toml::Table, name: &str) -> Result<String, ValidationProblem> {
    let value = values.remove(name).ok_or_else(|| {
        validation_problem(ValidationCode::MissingField, format!("missing {name}"))
    })?;
    let value = value.as_str().ok_or_else(|| {
        validation_problem(
            ValidationCode::InvalidField,
            format!("{name} must be a string"),
        )
    })?;
    validate_non_empty_text(value, name)?;
    Ok(value.to_owned())
}

fn optional_string(
    values: &mut toml::Table,
    name: &str,
) -> Result<Option<String>, ValidationProblem> {
    let Some(value) = values.remove(name) else {
        return Ok(None);
    };
    let value = value.as_str().ok_or_else(|| {
        validation_problem(
            ValidationCode::InvalidField,
            format!("{name} must be a string"),
        )
    })?;
    validate_non_empty_text(value, name)?;
    Ok(Some(value.to_owned()))
}

fn validate_non_empty_text(value: &str, name: &str) -> Result<(), ValidationProblem> {
    if value.is_empty() || value.contains('\0') {
        return Err(validation_problem(
            ValidationCode::InvalidField,
            format!("{name} must be non-empty and contain no NUL characters"),
        ));
    }
    Ok(())
}

fn validate_git_short_name(value: &str, name: &str) -> Result<(), ValidationProblem> {
    if !is_valid_git_short_name(value) {
        return Err(validation_problem(
            ValidationCode::InvalidField,
            format!("{name} must be a syntactically valid Git name"),
        ));
    }
    Ok(())
}

fn validate_primary_branch_name(value: &str) -> Result<(), ValidationProblem> {
    validate_git_short_name(value, "primary_branch")?;
    if value == "HEAD" {
        return Err(validation_problem(
            ValidationCode::InvalidField,
            "primary_branch must not be the reserved Git reference HEAD",
        ));
    }
    Ok(())
}

fn is_valid_git_short_name(value: &str) -> bool {
    !value.is_empty()
        && !value.starts_with(['-', '.'])
        && !value.ends_with(['.', '/'])
        && value != "@"
        && !value.contains("..")
        && !value.contains("@{")
        && !value
            .chars()
            .any(|character| character.is_control() || character.is_whitespace())
        && !value.contains(['~', '^', ':', '?', '*', '[', '\\'])
        && value.split('/').all(|component| {
            !component.is_empty() && !component.starts_with('.') && !component.ends_with(".lock")
        })
}

fn validation_problem(code: ValidationCode, message: impl Into<String>) -> ValidationProblem {
    ValidationProblem {
        path: PathBuf::from(CONFIG_PATH),
        code,
        message: message.into(),
    }
}
