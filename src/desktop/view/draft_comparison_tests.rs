//! The target-state labels and the comparison's rows, without a window.
//!
//! Draft types derive no `Debug`; rows are read through their text.

use std::collections::BTreeSet;

use manyhands::{
    drafts::{Change, DraftFields, TargetState, compare},
    results::ResultCode,
};

use super::{
    ALREADY_SAVED, CHANGED, CHANGED_CONTEXT, CURRENT, DRAFT_BEGAN, DraftComparisonView,
    ITEM_NOT_FOUND, ITEM_NOW, NONCONFORMING, PARENT_NOT_FOUND, READ_FAILED, REPOSITORY_UNAVAILABLE,
    TICKET_CLOSED, UNVERIFIED, YOUR_DRAFT, change_label, target_label,
};

fn ticket(title: &str, status: &str, deps: &[&str], body: &str) -> DraftFields {
    DraftFields::Ticket {
        title: title.to_owned(),
        ticket_type: "task".to_owned(),
        status: status.to_owned(),
        project: None,
        team: Some("core".to_owned()),
        deps: deps.iter().map(|dep| (*dep).to_owned()).collect(),
        parent: None,
        body: body.to_owned(),
    }
}

/// Every target state has its own text, the design's.
#[test]
fn every_target_state_has_its_own_label() {
    let fields = ticket("A", "open", &[], "Body");
    let comparison = compare(&fields, &fields, &fields);
    let states = [
        (TargetState::Current, CURRENT),
        (
            TargetState::Changed {
                theirs: fields.clone(),
                comparison: comparison.clone(),
                context_changed: false,
            },
            CHANGED,
        ),
        (
            TargetState::Changed {
                theirs: fields.clone(),
                comparison: comparison.clone(),
                context_changed: true,
            },
            CHANGED_CONTEXT,
        ),
        (TargetState::AlreadySaved, ALREADY_SAVED),
        (
            TargetState::Unverified {
                theirs: fields.clone(),
                comparison,
            },
            UNVERIFIED,
        ),
        (TargetState::TicketClosed, TICKET_CLOSED),
        (TargetState::ItemNotFound, ITEM_NOT_FOUND),
        (TargetState::Nonconforming, NONCONFORMING),
        (
            TargetState::RepositoryUnavailable(ResultCode::RepositoryNotRegistered),
            REPOSITORY_UNAVAILABLE,
        ),
        (TargetState::ReadFailed(ResultCode::Busy), READ_FAILED),
        (TargetState::ParentNotFound, PARENT_NOT_FOUND),
    ];
    let labels: Vec<&str> = states
        .iter()
        .map(|(state, expected)| {
            let label = target_label(state);
            assert_eq!(label, *expected);
            label
        })
        .collect();
    let distinct: BTreeSet<&str> = labels.iter().copied().collect();
    assert_eq!(distinct.len(), labels.len(), "{labels:?}");
    assert_eq!(CURRENT, "The item is unchanged since this draft began");
    assert_eq!(CHANGED, "The item changed since this draft began");
    assert_eq!(
        CHANGED_CONTEXT,
        "The item changed since this draft began and is now read from another copy"
    );
    assert_eq!(ALREADY_SAVED, "The item already holds this draft's text");
    assert_eq!(
        UNVERIFIED,
        "The item may have changed since this draft was saved"
    );
    assert_eq!(TICKET_CLOSED, "The ticket is closed");
    assert_eq!(ITEM_NOT_FOUND, "The item is gone");
    assert_eq!(NONCONFORMING, "The item's file is not a valid item");
    assert_eq!(REPOSITORY_UNAVAILABLE, "The repository is not available");
    assert_eq!(READ_FAILED, "The item could not be read");
    assert_eq!(PARENT_NOT_FOUND, "The comment it replies to is gone");
}

/// Each change reads as who made it.
#[test]
fn each_change_has_its_own_text() {
    let changes = [
        Change::None,
        Change::Mine,
        Change::Theirs,
        Change::BothSame,
        Change::Both,
    ];
    let labels: BTreeSet<&str> = changes.into_iter().map(change_label).collect();
    assert_eq!(labels.len(), changes.len());
    assert_eq!(change_label(Change::Mine), YOUR_DRAFT);
    assert_eq!(change_label(Change::Theirs), ITEM_NOW);
}

/// A row per field, in the comparison's order, with the draft's value,
/// the item's now and the base's, and the change as text.
#[test]
fn the_comparison_has_a_row_per_field_with_three_values() {
    let base = ticket("A", "open", &["X"], "Body");
    let theirs = ticket("A", "closed", &["X"], "Body");
    let mine = ticket("B", "open", &[], "Body");
    let comparison = compare(&base, &theirs, &mine);
    let view = DraftComparisonView::new(&comparison, &base, &theirs, &mine);
    let rows: Vec<(String, String, String, String, String)> = view
        .rows()
        .map(|row| {
            (
                row.field.to_owned(),
                row.change.to_owned(),
                row.mine.clone(),
                row.theirs.clone(),
                row.base.clone(),
            )
        })
        .collect();
    let row = |field: &str| rows.iter().find(|row| row.0 == field).unwrap().clone();
    assert_eq!(
        rows.iter().map(|row| row.0.as_str()).collect::<Vec<_>>(),
        [
            "Title",
            "Type",
            "Status",
            "Project",
            "Team",
            "Dependencies",
            "Parent",
            "Body"
        ]
    );
    assert_eq!(
        row("Title"),
        (
            "Title".into(),
            YOUR_DRAFT.into(),
            "B".into(),
            "A".into(),
            "A".into()
        )
    );
    assert_eq!(row("Status").1, ITEM_NOW);
    assert_eq!(row("Status").3, "closed");
    assert_eq!(row("Dependencies").2, "none");
    assert_eq!(row("Dependencies").4, "X");
    assert_eq!(row("Project").2, "none");
    assert_eq!(row("Body").1, change_label(Change::None));
    assert_eq!(DRAFT_BEGAN, "When the draft began");
}
