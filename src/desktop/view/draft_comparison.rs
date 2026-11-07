//! A draft's target state in words, its own fields read-only, and its
//! three-way comparison: one row per field with the draft's value, the
//! item's now and the base's, and who changed it. The tab's restored
//! draft section and Compare show them, and so does the Drafts page's
//! Inspect. It knows nothing of where it is drawn.
//!
//! It shows draft text and is drawn only on a draft's own surfaces; the
//! comparison holds that text and derives no `Debug`.

use gpui_kit::{
    App, InteractiveElement, IntoElement, ParentElement, StatefulInteractiveElement, Styled,
    TestSupportExt,
    component::{ActiveTheme, h_flex, v_flex},
    div, px,
};
use manyhands::drafts::{Change, Comparison, DraftFields, Field, TargetState};

pub(crate) const CURRENT: &str = "The item is unchanged since this draft began";
pub(crate) const CHANGED: &str = "The item changed since this draft began";
pub(crate) const CHANGED_CONTEXT: &str =
    "The item changed since this draft began and is now read from another copy";
pub(crate) const ALREADY_SAVED: &str = "The item already holds this draft's text";
pub(crate) const UNVERIFIED: &str = "The item may have changed since this draft was saved";
pub(crate) const TICKET_CLOSED: &str = "The ticket is closed";
pub(crate) const ITEM_NOT_FOUND: &str = "The item is gone";
/// A new item's draft whose item is not created yet (R-20).
pub(crate) const NOT_CREATED: &str = "The item is not created yet";
pub(crate) const NONCONFORMING: &str = "The item's file is not a valid item";
pub(crate) const REPOSITORY_UNAVAILABLE: &str = "The repository is not available";
pub(crate) const READ_FAILED: &str = "The item could not be read";
pub(crate) const PARENT_NOT_FOUND: &str = "The comment it replies to is gone";

pub(crate) const YOUR_DRAFT: &str = "Your draft";
pub(crate) const ITEM_NOW: &str = "The item now";
pub(crate) const DRAFT_BEGAN: &str = "When the draft began";
const UNCHANGED: &str = "Unchanged";
const BOTH_SAME: &str = "Both, the same way";
const BOTH: &str = "Both, differently";
const NONE: &str = "none";

/// Where the item stands, in words: shared by the tab and the Drafts page.
pub(crate) fn target_label(state: &TargetState) -> &'static str {
    match state {
        TargetState::Current => CURRENT,
        TargetState::Changed {
            context_changed: false,
            ..
        } => CHANGED,
        TargetState::Changed {
            context_changed: true,
            ..
        } => CHANGED_CONTEXT,
        TargetState::AlreadySaved => ALREADY_SAVED,
        TargetState::Unverified { .. } => UNVERIFIED,
        TargetState::TicketClosed => TICKET_CLOSED,
        TargetState::ItemNotFound => ITEM_NOT_FOUND,
        TargetState::Nonconforming => NONCONFORMING,
        TargetState::RepositoryUnavailable(_) => REPOSITORY_UNAVAILABLE,
        TargetState::ReadFailed(_) => READ_FAILED,
        TargetState::ParentNotFound => PARENT_NOT_FOUND,
        TargetState::NotCreated => NOT_CREATED,
    }
}

/// Who changed a field since the draft began.
pub(crate) fn change_label(change: Change) -> &'static str {
    match change {
        Change::None => UNCHANGED,
        Change::Mine => YOUR_DRAFT,
        Change::Theirs => ITEM_NOW,
        Change::BothSame => BOTH_SAME,
        Change::Both => BOTH,
    }
}

/// A field's name, in the comparison and in a draft's own section.
pub(crate) fn field_name(field: Field) -> &'static str {
    match field {
        Field::Title => "Title",
        Field::Type => "Type",
        Field::Status => "Status",
        Field::Project => "Project",
        Field::Team => "Team",
        Field::Deps => "Dependencies",
        Field::Parent => "Parent",
        Field::Body => "Body",
    }
}

/// A field's value as shown; "none" for one that is empty or absent.
pub(crate) fn field_value(fields: &DraftFields, field: Field) -> String {
    let optional = |value: &Option<String>| value.clone().unwrap_or_else(|| NONE.to_owned());
    match (fields, field) {
        (DraftFields::Document { title, .. }, Field::Title)
        | (DraftFields::Ticket { title, .. }, Field::Title) => title.clone(),
        (DraftFields::Document { body, .. }, Field::Body)
        | (DraftFields::Ticket { body, .. }, Field::Body)
        | (DraftFields::Comment { body }, Field::Body) => body.clone(),
        (DraftFields::Ticket { ticket_type, .. }, Field::Type) => ticket_type.clone(),
        (DraftFields::Ticket { status, .. }, Field::Status) => status.clone(),
        (DraftFields::Ticket { project, .. }, Field::Project) => optional(project),
        (DraftFields::Ticket { team, .. }, Field::Team) => optional(team),
        (DraftFields::Ticket { parent, .. }, Field::Parent) => optional(parent),
        (DraftFields::Ticket { deps, .. }, Field::Deps) if deps.is_empty() => NONE.to_owned(),
        (DraftFields::Ticket { deps, .. }, Field::Deps) => deps.join(", "),
        _ => NONE.to_owned(),
    }
}

/// One field of the comparison.
pub(crate) struct ComparisonRow {
    pub field: &'static str,
    pub change: &'static str,
    pub mine: String,
    pub theirs: String,
    pub base: String,
}

/// A comparison, ready to draw: a row per field in the comparison's order.
pub(crate) struct DraftComparisonView {
    rows: Vec<ComparisonRow>,
}

impl DraftComparisonView {
    /// `base` is what the draft began from, `theirs` the item now and
    /// `mine` the draft.
    pub(crate) fn new(
        comparison: &Comparison,
        base: &DraftFields,
        theirs: &DraftFields,
        mine: &DraftFields,
    ) -> Self {
        let rows = comparison
            .fields
            .iter()
            .map(|(field, change)| ComparisonRow {
                field: field_name(*field),
                change: change_label(*change),
                mine: field_value(mine, *field),
                theirs: field_value(theirs, *field),
                base: field_value(base, *field),
            })
            .collect();
        Self { rows }
    }

    #[cfg(test)]
    pub(crate) fn rows(&self) -> impl Iterator<Item = &ComparisonRow> {
        self.rows.iter()
    }

    /// The rows. Each piece of text is observable by its row: the field
    /// (`comparison-field`), the change (`comparison-change`) and the
    /// three values (`comparison-mine`, `-theirs`, `-base`).
    pub(crate) fn render(self, cx: &App) -> impl IntoElement + use<> {
        let muted = cx.theme().muted_foreground;
        let border = cx.theme().border;
        let value = move |id: (&'static str, usize), name: &'static str, value: String| {
            h_flex()
                .gap_2()
                .child(div().text_xs().text_color(muted).w(px(160.)).child(name))
                .child(text(id, value).text_sm())
        };
        v_flex().id("comparison").test_support().gap_2().children(
            self.rows.into_iter().enumerate().map(|(ix, row)| {
                v_flex()
                    .gap_1()
                    .p_2()
                    .border_1()
                    .border_color(border)
                    .child(
                        h_flex()
                            .gap_2()
                            .child(text(("comparison-field", ix), row.field).text_sm())
                            .child(
                                text(("comparison-change", ix), row.change)
                                    .text_xs()
                                    .text_color(muted),
                            ),
                    )
                    .child(value(("comparison-mine", ix), YOUR_DRAFT, row.mine))
                    .child(value(("comparison-theirs", ix), ITEM_NOW, row.theirs))
                    .child(value(("comparison-base", ix), DRAFT_BEGAN, row.base))
            }),
        )
    }
}

/// The ticket fields a draft's own fields name, in order, between its
/// title and its body.
const NAMED: [Field; 6] = [
    Field::Type,
    Field::Status,
    Field::Project,
    Field::Team,
    Field::Deps,
    Field::Parent,
];

/// The element IDs of a draft's own fields: its title, each named field
/// by its index, and its body.
pub(crate) struct FieldIds {
    pub title: &'static str,
    pub field: &'static str,
    pub body: &'static str,
}

/// A draft's own fields, read-only: its title, a ticket's named fields,
/// and its body. The tab's restored draft section and the Drafts page's
/// Inspect both show them, each under its own IDs.
pub(crate) fn draft_fields(
    ids: FieldIds,
    fields: &DraftFields,
    cx: &App,
) -> impl IntoElement + use<> {
    let named: &[Field] = match fields {
        DraftFields::Ticket { .. } => &NAMED,
        _ => &[],
    };
    v_flex()
        .gap_1()
        .child(labeled(ids.title, field_value(fields, Field::Title)).text_lg())
        .children(named.iter().enumerate().map(|(ix, field)| {
            text(
                (ids.field, ix),
                format!("{}: {}", field_name(*field), field_value(fields, *field)),
            )
            .text_sm()
        }))
        .child(
            labeled(ids.body, field_value(fields, Field::Body))
                .text_sm()
                .p_2()
                .border_1()
                .border_color(cx.theme().border),
        )
}

fn labeled(id: &'static str, value: String) -> impl IntoElement + Styled + use<> {
    div()
        .id(id)
        .test_support()
        .aria_label(value.clone())
        .child(value)
}

fn text(
    id: (&'static str, usize),
    value: impl Into<String>,
) -> impl IntoElement + Styled + InteractiveElement {
    let value = value.into();
    div()
        .id(id)
        .test_support()
        .aria_label(value.clone())
        .child(value)
}

#[cfg(test)]
#[path = "draft_comparison_tests.rs"]
mod tests;
