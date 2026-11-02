//! The stale-edit review's own model (design, Stale-edit review; decision
//! 4; R-44). On opening it takes three values for every field, the body
//! included: mine, the draft's current; theirs, the item as read again,
//! normalized as a base holds it (R-11); and the base, the draft's own.
//!
//! - A field changed only by theirs defaults to theirs, one changed only
//!   by mine to mine; one no one changed, or both changed alike, needs no
//!   choice. One changed by both waits for one.
//! - The body's chosen text is its result, which the user may then edit.
//!   Switching its choice once the result holds edits asks first.
//! - The choices are ready once every both-changed field has one; they
//!   give mine with each field taken from the side chosen, and the body's
//!   result.
//!
//! Choosing writes nothing: the model holds its texts until Use these
//! choices. It holds draft text, so it derives nothing.

use manyhands::drafts::{Change, DraftBase, DraftFields, Field, compare};

/// The review's words (`06-language-and-states.md`; the design's own).
pub(crate) const REVIEW_TITLE: &str = "Review changes";
pub(crate) const REVIEW_INTRO: &str = "This item changed since your edits began. Choose what to keep \
     for each field. Nothing is saved until you use your choices and save.";
pub(crate) const BEFORE: &str = "Before";
pub(crate) const THEIRS: &str = "Theirs";
pub(crate) const MINE: &str = "Mine";
pub(crate) const TAKE_THEIRS: &str = "Take theirs";
pub(crate) const KEEP_MINE: &str = "Keep mine";
pub(crate) const CHOOSE_ONE: &str = "Choose one";
pub(crate) const TAKING_THEIRS: &str = "Taking theirs";
pub(crate) const KEEPING_MINE: &str = "Keeping mine";
pub(crate) const RESULT: &str = "Result";
pub(crate) const CHOOSE_A_BODY: &str = "Take theirs or keep mine to edit the result.";
pub(crate) const USE_THESE_CHOICES: &str = "Use these choices";
pub(crate) const CHOOSE_EACH: &str = "Choose one for each field both of you changed.";
pub(crate) const READING: &str = "Reading the current version\u{2026}";
pub(crate) const NOT_READ: &str = "Could not read the current version. Your edits are kept.";
pub(crate) const REPLACE_YOUR_EDITS: &str = "Replace your edits?";
pub(crate) const REPLACE_DETAIL: &str =
    "Your edits to the result are replaced by the body you chose.";
pub(crate) const REPLACE: &str = "Replace";
pub(crate) const KEEP_MY_EDITS: &str = "Keep my edits";
/// The close dialog's note while the review its Save opened waits behind
/// it (Task 18's M4).
pub(crate) const CANCEL_TO_REVIEW: &str = "Cancel to review the changes.";

/// Which side a field takes.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Choice {
    Theirs,
    Mine,
}

/// What choosing the body came to.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum BodyChoice {
    /// The body takes the side chosen, which is now the result.
    Chosen,
    /// The result holds edits the choice would replace: nothing changed,
    /// and the caller asks first ("Replace your edits?").
    AskFirst,
}

/// The review: the three values, the comparison, the choices and the
/// body's result.
pub(crate) struct Review {
    base: DraftFields,
    theirs: DraftBase,
    mine: DraftFields,
    rows: Vec<(Field, Change)>,
    choices: Vec<Option<Choice>>,
    result: Option<String>,
}

impl Review {
    /// The review of `mine` over `base` against `theirs`, the item as read.
    pub(crate) fn new(base: DraftFields, theirs: DraftBase, mine: DraftFields) -> Self {
        let rows = compare(&base, &theirs.fields, &mine).fields;
        let choices = rows.iter().map(|(_, change)| default(*change)).collect();
        let mut review = Self {
            base,
            theirs,
            mine,
            rows,
            choices,
            result: None,
        };
        if let Some(choice) = review.body_row().and_then(|ix| review.choices[ix]) {
            review.result = Some(review.body(choice).to_owned());
        }
        review
    }

    /// Every field any of the three has, in the comparison's order, with
    /// who changed it.
    pub(crate) fn rows(&self) -> &[(Field, Change)] {
        &self.rows
    }

    /// The draft's base fields.
    pub(crate) fn base(&self) -> &DraftFields {
        &self.base
    }

    /// The item as read, normalized, with its observation: the base Use
    /// these choices goes on.
    pub(crate) fn theirs(&self) -> &DraftBase {
        &self.theirs
    }

    /// The draft's current fields.
    pub(crate) fn mine(&self) -> &DraftFields {
        &self.mine
    }

    /// The side row `ix` takes; `None` while a both-changed field waits.
    pub(crate) fn choice(&self, ix: usize) -> Option<Choice> {
        self.choices.get(ix).copied().flatten()
    }

    /// Row `ix`, a field other than the body, takes `choice`. The body is
    /// `choose_body`'s.
    pub(crate) fn choose(&mut self, ix: usize, choice: Choice) {
        if Some(ix) == self.body_row() {
            return;
        }
        if let Some(slot) = self.choices.get_mut(ix) {
            *slot = Some(choice);
        }
    }

    /// The body takes `choice`, which becomes the result, unless the
    /// result holds edits: then nothing changes, and the caller asks first.
    /// The side already chosen keeps its result, edits and all.
    pub(crate) fn choose_body(&mut self, choice: Choice) -> BodyChoice {
        if self.body_choice() == Some(choice) {
            return BodyChoice::Chosen;
        }
        if self.body_edited() {
            return BodyChoice::AskFirst;
        }
        self.replace_body(choice);
        BodyChoice::Chosen
    }

    /// The body takes `choice`, its result replaced, edits and all.
    pub(crate) fn replace_body(&mut self, choice: Choice) {
        let Some(ix) = self.body_row() else {
            return;
        };
        self.choices[ix] = Some(choice);
        self.result = Some(self.body(choice).to_owned());
    }

    /// The body's result, as the user edited it in the review's editor.
    /// It is kept once the body has a side.
    pub(crate) fn edit_result(&mut self, text: String) {
        if self.result.is_some() {
            self.result = Some(text);
        }
    }

    /// The body's result; `None` until the body has a side.
    pub(crate) fn result(&self) -> Option<&str> {
        self.result.as_deref()
    }

    /// The body's text on `choice`'s side.
    pub(crate) fn body(&self, choice: Choice) -> &str {
        let fields = match choice {
            Choice::Theirs => &self.theirs.fields,
            Choice::Mine => &self.mine,
        };
        body_of(fields)
    }

    /// The body's row.
    pub(crate) fn body_row(&self) -> Option<usize> {
        self.rows
            .iter()
            .position(|(field, _)| *field == Field::Body)
    }

    /// Whether the result differs from the body chosen: the user edited it.
    pub(crate) fn body_edited(&self) -> bool {
        match (self.body_choice(), self.result.as_deref()) {
            (Some(choice), Some(result)) => result != self.body(choice),
            _ => false,
        }
    }

    /// Whether every both-changed field has a choice.
    pub(crate) fn ready(&self) -> bool {
        self.choices.iter().all(Option::is_some)
    }

    /// The fields the choices give: mine, each field from the side chosen,
    /// and the body's result. `None` until ready.
    pub(crate) fn chosen(&self) -> Option<DraftFields> {
        if !self.ready() {
            return None;
        }
        let mut chosen = self.mine.clone();
        for ((field, _), choice) in self.rows.iter().zip(&self.choices) {
            if *field != Field::Body && *choice == Some(Choice::Theirs) {
                take(*field, &mut chosen, &self.theirs.fields);
            }
        }
        if let Some(result) = &self.result {
            set_body(&mut chosen, result.clone());
        }
        Some(chosen)
    }

    fn body_choice(&self) -> Option<Choice> {
        self.body_row().and_then(|ix| self.choices[ix])
    }
}

/// The base the review compares, normalized as theirs is (Part C fix 1,
/// M1; contract 8). A base taken from the file as it was, or kept from the
/// library's read after a save (fact 81), holds values the read's
/// normalization changes. Each field of `base` that equals `raw`'s, the
/// read as the file has it, takes `theirs`' value, the read normalized as
/// a base holds it (R-11, R-72), so that only a change of the item reads
/// as theirs. It is the dual comparison `adopted_base` applies to the
/// whole base. Fields of another kind of item are left as they are.
pub(crate) fn base_as_read(
    base: &DraftFields,
    raw: &DraftFields,
    theirs: &DraftFields,
) -> DraftFields {
    fn same<T: PartialEq + Clone>(value: &mut T, raw: &T, theirs: &T) {
        if value == raw {
            value.clone_from(theirs);
        }
    }
    let mut normalized = base.clone();
    match (&mut normalized, raw, theirs) {
        (
            DraftFields::Document { title, body },
            DraftFields::Document {
                title: raw_title,
                body: raw_body,
            },
            DraftFields::Document {
                title: their_title,
                body: their_body,
            },
        ) => {
            same(title, raw_title, their_title);
            same(body, raw_body, their_body);
        }
        (
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
                title: raw_title,
                ticket_type: raw_type,
                status: raw_status,
                project: raw_project,
                team: raw_team,
                deps: raw_deps,
                parent: raw_parent,
                body: raw_body,
            },
            DraftFields::Ticket {
                title: their_title,
                ticket_type: their_type,
                status: their_status,
                project: their_project,
                team: their_team,
                deps: their_deps,
                parent: their_parent,
                body: their_body,
            },
        ) => {
            same(title, raw_title, their_title);
            same(ticket_type, raw_type, their_type);
            same(status, raw_status, their_status);
            same(project, raw_project, their_project);
            same(team, raw_team, their_team);
            same(deps, raw_deps, their_deps);
            same(parent, raw_parent, their_parent);
            same(body, raw_body, their_body);
        }
        (
            DraftFields::Comment { body },
            DraftFields::Comment { body: raw_body },
            DraftFields::Comment { body: their_body },
        ) => same(body, raw_body, their_body),
        _ => {}
    }
    normalized
}

/// The choice a field starts with: theirs for their change alone, mine for
/// mine, and mine where both sides hold the same; none where both changed
/// it differently.
fn default(change: Change) -> Option<Choice> {
    match change {
        Change::Theirs => Some(Choice::Theirs),
        Change::Mine | Change::None | Change::BothSame => Some(Choice::Mine),
        Change::Both => None,
    }
}

fn body_of(fields: &DraftFields) -> &str {
    match fields {
        DraftFields::Document { body, .. }
        | DraftFields::Ticket { body, .. }
        | DraftFields::Comment { body } => body,
    }
}

fn set_body(fields: &mut DraftFields, text: String) {
    match fields {
        DraftFields::Document { body, .. }
        | DraftFields::Ticket { body, .. }
        | DraftFields::Comment { body } => *body = text,
    }
}

/// `field` of `into` takes `from`'s value. Fields of another kind of item
/// are left as they are.
fn take(field: Field, into: &mut DraftFields, from: &DraftFields) {
    match (into, from) {
        (
            DraftFields::Document { title, .. },
            DraftFields::Document {
                title: theirs_title,
                ..
            },
        ) => {
            if field == Field::Title {
                title.clone_from(theirs_title);
            }
        }
        (
            DraftFields::Ticket {
                title,
                ticket_type,
                status,
                project,
                team,
                deps,
                parent,
                ..
            },
            DraftFields::Ticket {
                title: their_title,
                ticket_type: their_type,
                status: their_status,
                project: their_project,
                team: their_team,
                deps: their_deps,
                parent: their_parent,
                ..
            },
        ) => match field {
            Field::Title => title.clone_from(their_title),
            Field::Type => ticket_type.clone_from(their_type),
            Field::Status => status.clone_from(their_status),
            Field::Project => project.clone_from(their_project),
            Field::Team => team.clone_from(their_team),
            Field::Deps => deps.clone_from(their_deps),
            Field::Parent => parent.clone_from(their_parent),
            Field::Body => {}
        },
        _ => {}
    }
}

#[cfg(test)]
#[path = "review_tests.rs"]
mod tests;
