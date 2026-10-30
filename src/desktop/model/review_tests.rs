//! The review's own model (R-44): the three values of every field, the
//! defaults one-sided changes take, the choices a both-changed field
//! waits for, the body's result and the fields Use these choices gives.
//! Draft fields hold text and derive no `Debug`: they are compared with
//! `assert!`.

use manyhands::drafts::{BaseContext, Change, ContextKind, DraftBase, DraftFields, Field};

use super::{BodyChoice, Choice, Review};

fn ticket(title: &str, status: &str, body: &str) -> DraftFields {
    DraftFields::Ticket {
        title: title.to_owned(),
        ticket_type: "task".to_owned(),
        status: status.to_owned(),
        project: None,
        team: None,
        deps: Vec::new(),
        parent: None,
        body: body.to_owned(),
    }
}

fn document(title: &str, body: &str) -> DraftFields {
    DraftFields::Document {
        title: title.to_owned(),
        body: body.to_owned(),
    }
}

/// The item as read, as a base: `fields`, with an observation.
fn read(fields: DraftFields) -> DraftBase {
    DraftBase {
        observation: Some("theirs".to_owned()),
        path: "tickets/one.md".to_owned(),
        context: BaseContext {
            kind: ContextKind::Active,
            branch: Some("manyhands/one".to_owned()),
            worktree: "/worktrees/one".to_owned(),
            head_oid: None,
        },
        source: Some("source".to_owned()),
        fields,
    }
}

fn row(review: &Review, field: Field) -> usize {
    review
        .rows()
        .iter()
        .position(|(at, _)| *at == field)
        .expect("the field is listed")
}

fn body(fields: &DraftFields) -> &str {
    match fields {
        DraftFields::Document { body, .. }
        | DraftFields::Ticket { body, .. }
        | DraftFields::Comment { body } => body,
    }
}

/// A field changed only by theirs defaults to theirs, one changed only by
/// mine to mine, and one changed by both waits for a choice; a field no
/// one changed, or both changed alike, needs none.
#[test]
fn review_defaults_follow_who_changed_each_field() {
    let base = ticket("Base", "open", "Base body.\n");
    let theirs = read(ticket("Theirs", "in progress", "Base body.\n"));
    let mine = ticket("Mine", "open", "My body.\n");
    let review = Review::new(base, theirs, mine);

    let title = row(&review, Field::Title);
    let status = row(&review, Field::Status);
    let kind = row(&review, Field::Type);
    let body_row = row(&review, Field::Body);
    assert_eq!(review.rows()[title].1, Change::Both);
    assert_eq!(review.choice(title), None, "both changed it: choose");
    assert_eq!(review.rows()[status].1, Change::Theirs);
    assert_eq!(review.choice(status), Some(Choice::Theirs));
    assert_eq!(review.rows()[kind].1, Change::None);
    assert!(review.choice(kind).is_some(), "no one changed it");
    assert_eq!(review.rows()[body_row].1, Change::Mine);
    assert_eq!(review.choice(body_row), Some(Choice::Mine));
    assert!(!review.ready(), "the title waits for a choice");
    assert!(review.chosen().is_none());
}

/// The chosen fields take each side as chosen, and the body's result.
#[test]
fn review_choices_give_each_side_and_the_bodys_result() {
    let base = document("Base", "Base body.\n");
    let theirs = read(document("Their title", "Their body.\n"));
    let mine = document("My title", "My body.\n");
    let mut review = Review::new(base, theirs, mine);
    let title = row(&review, Field::Title);
    let body_row = row(&review, Field::Body);
    assert!(review.result().is_none(), "no body until one is chosen");

    review.choose(title, Choice::Theirs);
    assert!(!review.ready(), "the body waits too");
    assert_eq!(review.choose_body(Choice::Mine), BodyChoice::Chosen);
    assert_eq!(review.choice(body_row), Some(Choice::Mine));
    assert!(review.result() == Some("My body.\n"));
    assert!(review.ready());
    let chosen = review.chosen().expect("every field is chosen");
    assert!(chosen == document("Their title", "My body.\n"));

    review.edit_result("My edited body.\n".to_owned());
    let chosen = review.chosen().expect("every field is chosen");
    assert!(
        body(&chosen) == "My edited body.\n",
        "the result, as edited"
    );
    assert!(review.theirs().observation.as_deref() == Some("theirs"));
}

/// Switching the body's choice after its result was edited asks first:
/// nothing changes until the caller replaces the edits.
#[test]
fn review_switching_the_body_after_an_edit_asks_first() {
    let base = document("Title", "Base body.\n");
    let theirs = read(document("Title", "Their body.\n"));
    let mine = document("Title", "My body.\n");
    let mut review = Review::new(base, theirs, mine);
    let body_row = row(&review, Field::Body);

    assert_eq!(review.choose_body(Choice::Theirs), BodyChoice::Chosen);
    assert_eq!(review.choose_body(Choice::Mine), BodyChoice::Chosen);
    assert!(
        review.result() == Some("My body.\n"),
        "no edit: no question"
    );
    assert_eq!(review.choose_body(Choice::Theirs), BodyChoice::Chosen);
    review.edit_result("Their body.\nmore".to_owned());
    assert!(review.body_edited());

    assert_eq!(review.choose_body(Choice::Mine), BodyChoice::AskFirst);
    assert_eq!(review.choice(body_row), Some(Choice::Theirs), "unchanged");
    assert!(review.result() == Some("Their body.\nmore"), "edits kept");

    review.replace_body(Choice::Mine);
    assert_eq!(review.choice(body_row), Some(Choice::Mine));
    assert!(review.result() == Some("My body.\n"));
    assert!(!review.body_edited());

    // Edited back to the chosen body: nothing to ask about.
    review.edit_result("x".to_owned());
    review.edit_result("My body.\n".to_owned());
    assert_eq!(review.choose_body(Choice::Theirs), BodyChoice::Chosen);
}

/// A body no one changed, or one changed by one side, has its result from
/// the start; the base's fields are the base given.
#[test]
fn review_a_body_with_a_default_has_its_result_from_the_start() {
    let base = document("Base", "Base body.\n");
    let theirs = read(document("Theirs", "Base body.\n"));
    let mine = document("Base", "Base body.\nmine");
    let review = Review::new(base, theirs, mine);
    assert!(review.result() == Some("Base body.\nmine"));
    assert!(review.base() == &document("Base", "Base body.\n"));
    assert!(review.mine() == &document("Base", "Base body.\nmine"));
    assert!(review.ready());
    let chosen = review.chosen().expect("ready");
    assert!(chosen == document("Theirs", "Base body.\nmine"));
}
