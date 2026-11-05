//! One canonical path's three-way choice (D5 design, B2; R-4, R-5, R-9).

use std::{mem, path::Path};

use manyhands::canonical::{CanonicalItem, parse_item, serialize_item};
use serde_yaml::{Mapping, Value};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

/// Keep mine or Keep theirs, for a field; also which side a field only one
/// side changed is taken from.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Choice {
    Mine,
    Theirs,
}

/// The body's choice: Keep mine, Keep theirs, Keep both (R-4), or the
/// result as the person edited it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum BodyChoice {
    Mine,
    Theirs,
    Both,
    Edited,
}

/// A field both sides changed, differently: by its front matter key, with
/// each side's value as text, `None` where the side has no value.
pub(crate) struct FieldPart {
    pub key: String,
    pub base: Option<String>,
    pub mine: Option<String>,
    pub theirs: Option<String>,
    pub choice: Option<Choice>,
    slot: Slot,
}

/// A field only one side changed, taken from that side (R-5).
pub(crate) struct TakenField {
    pub key: String,
    pub from: Choice,
    slot: Slot,
}

/// The body, when both sides changed it, differently.
pub(crate) struct BodyPart {
    pub base: String,
    pub mine: String,
    pub theirs: String,
    pub choice: Option<BodyChoice>,
    /// The Result editor's text.
    pub result: String,
}

/// A path's result.
pub(crate) enum PathResult {
    /// A part is not chosen yet.
    Pending,
    /// The whole text to submit, and whether it is written in the standard
    /// layout rather than as mine's bytes (R-9).
    Ready { source: String, normalized: bool },
    /// The chosen values do not make an item Manyhands can write.
    Invalid,
}

/// One canonical path of a conflict.
pub(crate) struct PathConflict {
    display_path: String,
    token: String,
    /// Mine's bytes, submitted as they are when nothing differs from mine.
    mine_source: String,
    mine: CanonicalItem,
    theirs: CanonicalItem,
    fields: Vec<FieldPart>,
    taken: Vec<TakenField>,
    body: Option<BodyPart>,
    /// Which side the body is taken from when only one changed it.
    body_taken: Option<Choice>,
}

/// A front matter key: one Manyhands defines for the kind, or one it keeps
/// as it is.
#[derive(Clone)]
enum Slot {
    Known(&'static str),
    Unknown(Value),
}

const DOCUMENT_KEYS: &[&str] = &["id", "title"];
const TICKET_KEYS: &[&str] = &[
    "id",
    "title",
    "type",
    "status",
    "project",
    "team",
    "closed_at",
    "closed_by",
];
const COMMENT_KEYS: &[&str] = &["id", "item_id", "parent_id", "created_at", "created_by"];

impl PathConflict {
    /// The path's parts, or `None` when a side is missing, `parse_item`
    /// rejects one at `display_path`, or the sides are not one kind.
    pub(super) fn parse(
        display_path: &str,
        token: &str,
        base: Option<&str>,
        mine: Option<&str>,
        theirs: Option<&str>,
    ) -> Option<Self> {
        let path = Path::new(display_path);
        let parse = |source: Option<&str>| parse_item(path, source?).ok();
        let (base_item, mine_item, theirs_item) = (parse(base)?, parse(mine)?, parse(theirs)?);
        let kind = mem::discriminant(&mine_item);
        if mem::discriminant(&base_item) != kind || mem::discriminant(&theirs_item) != kind {
            return None;
        }
        let mut fields = Vec::new();
        let mut taken = Vec::new();
        for slot in slots(&base_item, &mine_item, &theirs_item) {
            let (b, m, t) = (
                value(&base_item, &slot),
                value(&mine_item, &slot),
                value(&theirs_item, &slot),
            );
            let key = slot_text(&slot);
            match three_way(&b, &m, &t) {
                Merge::Same => {}
                Merge::Taken(from) => taken.push(TakenField { key, from, slot }),
                Merge::Choose => fields.push(FieldPart {
                    key,
                    base: b.as_ref().map(value_text),
                    mine: m.as_ref().map(value_text),
                    theirs: t.as_ref().map(value_text),
                    choice: None,
                    slot,
                }),
            }
        }
        let (b, m, t) = (body(&base_item), body(&mine_item), body(&theirs_item));
        let (body_part, body_taken) = match three_way(&b, &m, &t) {
            Merge::Same => (None, None),
            Merge::Taken(from) => (None, Some(from)),
            Merge::Choose => (
                Some(BodyPart {
                    base: b.to_owned(),
                    mine: m.to_owned(),
                    theirs: t.to_owned(),
                    choice: None,
                    result: m.to_owned(),
                }),
                None,
            ),
        };
        Some(Self {
            display_path: display_path.to_owned(),
            token: token.to_owned(),
            mine_source: mine?.to_owned(),
            mine: mine_item,
            theirs: theirs_item,
            fields,
            taken,
            body: body_part,
            body_taken,
        })
    }

    pub(crate) fn display_path(&self) -> &str {
        &self.display_path
    }

    pub(crate) fn token(&self) -> &str {
        &self.token
    }

    /// The fields to choose between, in the canonical order, then the
    /// keys Manyhands does not define.
    pub(crate) fn fields(&self) -> &[FieldPart] {
        &self.fields
    }

    /// The fields taken from the one side that changed them, in the same
    /// order.
    pub(crate) fn taken(&self) -> &[TakenField] {
        &self.taken
    }

    pub(crate) fn body(&self) -> Option<&BodyPart> {
        self.body.as_ref()
    }

    /// Which side the body is taken from when only that side changed it
    /// (R-5), for the columns; `None` when it is a part, or neither side
    /// changed it.
    pub(crate) fn body_taken(&self) -> Option<Choice> {
        self.body_taken
    }

    pub(crate) fn choose_field(&mut self, index: usize, choice: Choice) {
        if let Some(field) = self.fields.get_mut(index) {
            field.choice = Some(choice);
        }
    }

    /// Keep mine, theirs or both sets the Result to that text; Edited keeps
    /// the Result as it is.
    pub(crate) fn choose_body(&mut self, choice: BodyChoice) {
        let Some(body) = &mut self.body else {
            return;
        };
        match choice {
            BodyChoice::Mine => body.result = body.mine.clone(),
            BodyChoice::Theirs => body.result = body.theirs.clone(),
            BodyChoice::Both => {
                body.result = format!("{}\n\n{}", body.mine.trim_end_matches('\n'), body.theirs)
            }
            BodyChoice::Edited => {}
        }
        body.choice = Some(choice);
    }

    /// The Result as the person typed it: the body is chosen.
    pub(crate) fn edit_body(&mut self, result: String) {
        if let Some(body) = &mut self.body {
            body.result = result;
            body.choice = Some(BodyChoice::Edited);
        }
    }

    /// Takes `earlier`'s choice of each part that has the same three sides
    /// here, and the body's result with it.
    pub(crate) fn keep_choices_from(&mut self, earlier: &PathConflict) {
        for field in &mut self.fields {
            if let Some(was) = earlier.fields.iter().find(|was| {
                was.key == field.key
                    && was.base == field.base
                    && was.mine == field.mine
                    && was.theirs == field.theirs
            }) {
                field.choice = was.choice;
            }
        }
        if let (Some(body), Some(was)) = (&mut self.body, &earlier.body)
            && (&was.base, &was.mine, &was.theirs) == (&body.base, &body.mine, &body.theirs)
        {
            body.choice = was.choice;
            body.result = was.result.clone();
        }
    }

    /// How many of the path's parts are chosen, of how many.
    pub(crate) fn chosen(&self) -> (usize, usize) {
        let fields = self.fields.iter().filter(|f| f.choice.is_some()).count();
        let body = self.body.as_ref().map(|body| body.choice.is_some());
        (
            fields + usize::from(body == Some(true)),
            self.fields.len() + usize::from(body.is_some()),
        )
    }

    /// Mine, with each field taken from theirs or chosen theirs, and the
    /// body's result. Mine's bytes when that is mine itself; otherwise the
    /// standard layout (R-9).
    pub(crate) fn result(&self) -> PathResult {
        let (chosen, of) = self.chosen();
        if chosen < of {
            return PathResult::Pending;
        }
        let mut result = self.mine.clone();
        let from_theirs = self
            .taken
            .iter()
            .filter(|field| field.from == Choice::Theirs)
            .map(|field| &field.slot)
            .chain(
                self.fields
                    .iter()
                    .filter(|field| field.choice == Some(Choice::Theirs))
                    .map(|field| &field.slot),
            );
        for slot in from_theirs {
            copy(&mut result, &self.theirs, slot);
        }
        if let Some(part) = &self.body {
            set_body(&mut result, part.result.clone());
        } else if self.body_taken == Some(Choice::Theirs) {
            set_body(&mut result, body(&self.theirs).to_owned());
        }
        if result == self.mine {
            return PathResult::Ready {
                source: self.mine_source.clone(),
                normalized: false,
            };
        }
        match serialize_item(&result) {
            Ok(source) => PathResult::Ready {
                source,
                normalized: true,
            },
            Err(_) => PathResult::Invalid,
        }
    }
}

enum Merge {
    Same,
    Taken(Choice),
    Choose,
}

fn three_way<T: PartialEq + ?Sized>(base: &T, mine: &T, theirs: &T) -> Merge {
    if mine == theirs {
        Merge::Same
    } else if base == mine {
        Merge::Taken(Choice::Theirs)
    } else if base == theirs {
        Merge::Taken(Choice::Mine)
    } else {
        Merge::Choose
    }
}

/// Every key of the three sides: the kind's own, in the canonical order,
/// then the others, mine's first, then theirs', then base's.
fn slots(base: &CanonicalItem, mine: &CanonicalItem, theirs: &CanonicalItem) -> Vec<Slot> {
    let known = match mine {
        CanonicalItem::Document(_) => DOCUMENT_KEYS,
        CanonicalItem::Ticket(_) => TICKET_KEYS,
        CanonicalItem::Comment(_) => COMMENT_KEYS,
    };
    let mut slots: Vec<Slot> = known.iter().map(|key| Slot::Known(key)).collect();
    let mut others: Vec<&Value> = Vec::new();
    for item in [mine, theirs, base] {
        for key in unknown(item).keys() {
            if !others.contains(&key) {
                others.push(key);
            }
        }
    }
    slots.extend(others.into_iter().cloned().map(Slot::Unknown));
    slots
}

fn slot_text(slot: &Slot) -> String {
    match slot {
        Slot::Known(key) => (*key).to_owned(),
        Slot::Unknown(key) => value_text(key),
    }
}

/// A value as the sheet shows it: a string as it is, anything else as YAML.
fn value_text(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        other => serde_yaml::to_string(other)
            .map(|text| text.trim_end().to_owned())
            .unwrap_or_default(),
    }
}

fn unknown(item: &CanonicalItem) -> &Mapping {
    match item {
        CanonicalItem::Document(document) => &document.unknown,
        CanonicalItem::Ticket(ticket) => &ticket.unknown,
        CanonicalItem::Comment(comment) => &comment.unknown,
    }
}

fn unknown_mut(item: &mut CanonicalItem) -> &mut Mapping {
    match item {
        CanonicalItem::Document(document) => &mut document.unknown,
        CanonicalItem::Ticket(ticket) => &mut ticket.unknown,
        CanonicalItem::Comment(comment) => &mut comment.unknown,
    }
}

fn body(item: &CanonicalItem) -> &str {
    match item {
        CanonicalItem::Document(document) => &document.body,
        CanonicalItem::Ticket(ticket) => &ticket.body,
        CanonicalItem::Comment(comment) => &comment.body,
    }
}

fn set_body(item: &mut CanonicalItem, text: String) {
    match item {
        CanonicalItem::Document(document) => document.body = text,
        CanonicalItem::Ticket(ticket) => ticket.body = text,
        CanonicalItem::Comment(comment) => comment.body = text,
    }
}

fn text(value: impl ToString) -> Option<Value> {
    Some(Value::String(value.to_string()))
}

fn optional(value: &Option<String>) -> Option<Value> {
    value.clone().map(Value::String)
}

fn time(value: OffsetDateTime) -> Option<Value> {
    value.format(&Rfc3339).ok().map(Value::String)
}

/// The side's value of `slot`, `None` where it has none.
fn value(item: &CanonicalItem, slot: &Slot) -> Option<Value> {
    let key = match slot {
        Slot::Unknown(key) => return unknown(item).get(key).cloned(),
        Slot::Known(key) => *key,
    };
    match (item, key) {
        (CanonicalItem::Document(document), "id") => text(&document.id),
        (CanonicalItem::Document(document), "title") => text(&document.title),
        (CanonicalItem::Ticket(ticket), "id") => text(&ticket.id),
        (CanonicalItem::Ticket(ticket), "title") => text(&ticket.title),
        (CanonicalItem::Ticket(ticket), "type") => text(&ticket.ticket_type),
        (CanonicalItem::Ticket(ticket), "status") => text(&ticket.status),
        (CanonicalItem::Ticket(ticket), "project") => optional(&ticket.project),
        (CanonicalItem::Ticket(ticket), "team") => optional(&ticket.team),
        (CanonicalItem::Ticket(ticket), "closed_at") => ticket.closed_at.and_then(time),
        (CanonicalItem::Ticket(ticket), "closed_by") => optional(&ticket.closed_by),
        (CanonicalItem::Comment(comment), "id") => text(&comment.id),
        (CanonicalItem::Comment(comment), "item_id") => text(&comment.item_id),
        (CanonicalItem::Comment(comment), "parent_id") => comment.parent_id.as_ref().and_then(text),
        (CanonicalItem::Comment(comment), "created_at") => time(comment.created_at),
        (CanonicalItem::Comment(comment), "created_by") => optional(&comment.created_by),
        _ => None,
    }
}

/// Puts `from`'s value of `slot` in `to`. The sides are one kind (`parse`).
fn copy(to: &mut CanonicalItem, from: &CanonicalItem, slot: &Slot) {
    let key = match slot {
        Slot::Unknown(key) => {
            match unknown(from).get(key) {
                Some(value) => {
                    unknown_mut(to).insert(key.clone(), value.clone());
                }
                None => {
                    unknown_mut(to).shift_remove(key);
                }
            }
            return;
        }
        Slot::Known(key) => *key,
    };
    match (to, from) {
        (CanonicalItem::Document(to), CanonicalItem::Document(from)) => match key {
            "id" => to.id = from.id.clone(),
            "title" => to.title = from.title.clone(),
            _ => {}
        },
        (CanonicalItem::Ticket(to), CanonicalItem::Ticket(from)) => match key {
            "id" => to.id = from.id.clone(),
            "title" => to.title = from.title.clone(),
            "type" => to.ticket_type = from.ticket_type.clone(),
            "status" => to.status = from.status.clone(),
            "project" => to.project = from.project.clone(),
            "team" => to.team = from.team.clone(),
            "closed_at" => to.closed_at = from.closed_at,
            "closed_by" => to.closed_by = from.closed_by.clone(),
            _ => {}
        },
        (CanonicalItem::Comment(to), CanonicalItem::Comment(from)) => match key {
            "id" => to.id = from.id.clone(),
            "item_id" => to.item_id = from.item_id.clone(),
            "parent_id" => to.parent_id = from.parent_id.clone(),
            "created_at" => to.created_at = from.created_at,
            "created_by" => to.created_by = from.created_by.clone(),
            _ => {}
        },
        _ => {}
    }
}
