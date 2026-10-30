//! The details sheet's named fields as pure decisions (design, Named
//! controls in the details sheet): which fields an item's kind names, a
//! field's value among a draft's fields and the fields with it changed,
//! the message of an empty required field, and the values suggested for a
//! field (R-57).
//!
//! - **Named fields.** A document's Title; a ticket's Title, Type, Status,
//!   Project and Team. Identity, short code, closure and every other key,
//!   a document's status, date and author included, are never edited here
//!   (decision 14).
//! - **Absent.** An emptied Project or Team is `None`, so a save sends it
//!   as absent and the library removes the key (R-27).
//! - **Required.** An empty Title, Type or Status blocks every save; its
//!   message is the reason. Empty is the library's own rule: no character
//!   at all (`canonical.rs`, `validate_item_text`).
//! - **Suggestions** are the distinct values on the tickets the desktop
//!   has loaded for the repository, since the Kit's combobox takes listed
//!   values only and no distinct-values read exists (fact 64). They are
//!   suggestions only: any typed value is kept.
//!
//! Field values are the user's text, so nothing here derives `Debug`.

use std::collections::BTreeSet;

use manyhands::drafts::DraftFields;

use crate::desktop::model::tickets::TicketRow;

pub(crate) const TICKET_NEEDS_TITLE: &str = "A ticket needs a title before it can be saved.";
pub(crate) const TICKET_NEEDS_TYPE: &str = "A ticket needs a type before it can be saved.";
pub(crate) const TICKET_NEEDS_STATUS: &str = "A ticket needs a status before it can be saved.";
pub(crate) const DOCUMENT_NEEDS_TITLE: &str = "A document needs a title before it can be saved.";

/// How many values a field's suggestions show at most.
const OFFERED: usize = 8;

/// A field the details sheet edits.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Field {
    Title,
    Type,
    Status,
    Project,
    Team,
}

impl Field {
    const TICKET: [Self; 5] = [
        Self::Title,
        Self::Type,
        Self::Status,
        Self::Project,
        Self::Team,
    ];
    const DOCUMENT: [Self; 1] = [Self::Title];

    /// The fields `fields`' kind names, in the sheet's order; a comment's
    /// none.
    pub(crate) fn of(fields: &DraftFields) -> &'static [Self] {
        match fields {
            DraftFields::Ticket { .. } => &Self::TICKET,
            DraftFields::Document { .. } => &Self::DOCUMENT,
            DraftFields::Comment { .. } => &[],
        }
    }

    /// Whether its values are suggested from the loaded tickets (R-57).
    pub(crate) fn suggested(self) -> bool {
        self != Self::Title
    }
}

/// `field`'s value among `fields`: `None` where their kind does not name
/// it, and `""` for an absent Project or Team.
pub(crate) fn value(fields: &DraftFields, field: Field) -> Option<&str> {
    match (fields, field) {
        (DraftFields::Document { title, .. }, Field::Title)
        | (DraftFields::Ticket { title, .. }, Field::Title) => Some(title),
        (DraftFields::Ticket { ticket_type, .. }, Field::Type) => Some(ticket_type),
        (DraftFields::Ticket { status, .. }, Field::Status) => Some(status),
        (DraftFields::Ticket { project, .. }, Field::Project) => {
            Some(project.as_deref().unwrap_or_default())
        }
        (DraftFields::Ticket { team, .. }, Field::Team) => {
            Some(team.as_deref().unwrap_or_default())
        }
        _ => None,
    }
}

/// `fields` with `field` holding `typed`, exactly as typed. An emptied
/// Project or Team is absent (R-27). A field their kind does not name
/// leaves them as they are.
pub(crate) fn with_value(fields: &DraftFields, field: Field, typed: &str) -> DraftFields {
    let mut fields = fields.clone();
    let optional = (!typed.is_empty()).then(|| typed.to_owned());
    match (&mut fields, field) {
        (DraftFields::Document { title, .. }, Field::Title)
        | (DraftFields::Ticket { title, .. }, Field::Title) => *title = typed.to_owned(),
        (DraftFields::Ticket { ticket_type, .. }, Field::Type) => *ticket_type = typed.to_owned(),
        (DraftFields::Ticket { status, .. }, Field::Status) => *status = typed.to_owned(),
        (DraftFields::Ticket { project, .. }, Field::Project) => *project = optional,
        (DraftFields::Ticket { team, .. }, Field::Team) => *team = optional,
        _ => {}
    }
    fields
}

/// The message of `field` when `fields` leave it empty and it is
/// required: a Title, Type or Status. `None` otherwise.
pub(crate) fn required_message(fields: &DraftFields, field: Field) -> Option<&'static str> {
    if !value(fields, field)?.is_empty() {
        return None;
    }
    match (fields, field) {
        (DraftFields::Document { .. }, Field::Title) => Some(DOCUMENT_NEEDS_TITLE),
        (DraftFields::Ticket { .. }, Field::Title) => Some(TICKET_NEEDS_TITLE),
        (DraftFields::Ticket { .. }, Field::Type) => Some(TICKET_NEEDS_TYPE),
        (DraftFields::Ticket { .. }, Field::Status) => Some(TICKET_NEEDS_STATUS),
        _ => None,
    }
}

/// Why `fields` cannot be saved: the message of the first required field,
/// in the sheet's order, that they leave empty.
pub(crate) fn missing(fields: &DraftFields) -> Option<&'static str> {
    Field::of(fields)
        .iter()
        .find_map(|field| required_message(fields, *field))
}

/// The distinct values of each suggested field (R-57).
#[derive(Default)]
pub(crate) struct Suggestions {
    ticket_type: BTreeSet<String>,
    status: BTreeSet<String>,
    project: BTreeSet<String>,
    team: BTreeSet<String>,
}

impl Suggestions {
    /// The values on `rows`, the tickets the desktop loaded.
    pub(crate) fn of_rows<'a>(rows: impl IntoIterator<Item = &'a TicketRow>) -> Self {
        let mut suggestions = Self::default();
        for row in rows {
            suggestions.add(Field::Type, row.ticket_type.as_deref());
            suggestions.add(Field::Status, row.status.as_deref());
            suggestions.add(Field::Project, row.project.as_deref());
            suggestions.add(Field::Team, row.team.as_deref());
        }
        suggestions
    }

    /// Adds `value`, a loaded ticket's `field`; an empty one is no value.
    pub(crate) fn add(&mut self, field: Field, value: Option<&str>) {
        let Some(value) = value.filter(|value| !value.is_empty()) else {
            return;
        };
        if let Some(values) = self.values_mut(field) {
            values.insert(value.to_owned());
        }
    }

    fn values_mut(&mut self, field: Field) -> Option<&mut BTreeSet<String>> {
        match field {
            Field::Title => None,
            Field::Type => Some(&mut self.ticket_type),
            Field::Status => Some(&mut self.status),
            Field::Project => Some(&mut self.project),
            Field::Team => Some(&mut self.team),
        }
    }

    fn values(&self, field: Field) -> Option<&BTreeSet<String>> {
        match field {
            Field::Title => None,
            Field::Type => Some(&self.ticket_type),
            Field::Status => Some(&self.status),
            Field::Project => Some(&self.project),
            Field::Team => Some(&self.team),
        }
    }

    /// The values offered for `field` while it holds `typed`, in order: all
    /// but `typed` while it is empty or one of them, else those that hold
    /// it, ignoring case. At most a handful.
    pub(crate) fn offered(&self, field: Field, typed: &str) -> Vec<String> {
        let Some(values) = self.values(field) else {
            return Vec::new();
        };
        let all = typed.is_empty() || values.contains(typed);
        let typed_lower = typed.to_lowercase();
        values
            .iter()
            .filter(|value| value.as_str() != typed)
            .filter(|value| all || value.to_lowercase().contains(&typed_lower))
            .take(OFFERED)
            .cloned()
            .collect()
    }
}
