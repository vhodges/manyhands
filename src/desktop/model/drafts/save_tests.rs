//! What a save's answer does to its draft, the base a save leaves, and the
//! relationship writes a ticket save sends (ruling R13).
//!
//! Draft types derive no `Debug`, so they are read through `match`.

use std::path::PathBuf;

use manyhands::{
    drafts::{BaseContext, ContextKind, DraftBase, DraftFields, DraftTarget, ItemKind},
    repository::{
        IndexState, IndexStateDto, ItemContextDto, ItemContextKind, ItemDto, ItemDtoKind,
        RelationshipWrite,
    },
    results::{CheckpointEffect, Outcome, ResultCode},
};

use super::{SaveVerdict, base_after_save, deps_write, parent_write, save_intent, save_verdict};
use crate::desktop::services::requests::Intent;

const ITEM: &str = "01ARZ3NDEKTSV4RRFFQ69G5FG0";
const OTHER: &str = "01ARZ3NDEKTSV4RRFFQ69G5FG1";

fn document(title: &str, body: &str) -> DraftFields {
    DraftFields::Document {
        title: title.to_owned(),
        body: body.to_owned(),
    }
}

fn ticket(deps: &[&str], parent: Option<&str>) -> DraftFields {
    DraftFields::Ticket {
        title: "Title".to_owned(),
        ticket_type: "task".to_owned(),
        status: "open".to_owned(),
        project: Some("Project".to_owned()),
        team: None,
        deps: deps.iter().map(|dep| (*dep).to_owned()).collect(),
        parent: parent.map(str::to_owned),
        body: "Body".to_owned(),
    }
}

fn base(fields: DraftFields) -> DraftBase {
    DraftBase {
        observation: Some("before".to_owned()),
        path: "docs/a.md".to_owned(),
        context: BaseContext {
            kind: ContextKind::Primary,
            branch: Some("main".to_owned()),
            worktree: "/repo".to_owned(),
            head_oid: None,
        },
        source: Some("old source".to_owned()),
        fields,
    }
}

/// A document as `show_item` gives it after the save, in its own item
/// context.
fn read(title: &str, body: &str) -> ItemDto {
    ItemDto {
        id: Some(ITEM.to_owned()),
        kind: ItemDtoKind::Document,
        path: "docs/b.md".to_owned(),
        title: Some(title.to_owned()),
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
        body: Some(body.to_owned()),
        source: Some("new source".to_owned()),
        observation: Some("after".to_owned()),
        context: ItemContextDto {
            kind: ItemContextKind::Active,
            branch: Some("item".to_owned()),
            worktree: "/repo/worktree".to_owned(),
            head_oid: Some("abc".to_owned()),
        },
        changed_at: None,
        change_source: None,
        problems: Vec::new(),
        index: IndexStateDto {
            state: IndexState::Current,
            refreshed_at: None,
        },
    }
}

/// Retire exactly for a committed checkpoint, or an unchanged one that
/// succeeded or found nothing to do; Pending for a pending checkpoint;
/// Keep for everything else.
#[test]
fn a_save_retires_exactly_when_its_checkpoint_is_committed_or_unchanged_and_done() {
    for outcome in Outcome::ALL {
        for checkpoint in CheckpointEffect::ALL {
            let expected = match (checkpoint, outcome) {
                (CheckpointEffect::Committed, _) => SaveVerdict::Retire,
                (CheckpointEffect::Unchanged, Outcome::Success | Outcome::Noop) => {
                    SaveVerdict::Retire
                }
                (CheckpointEffect::Pending, _) => SaveVerdict::Pending,
                _ => SaveVerdict::Keep,
            };
            assert_eq!(
                save_verdict(outcome, checkpoint),
                expected,
                "{} with {}",
                outcome.as_str(),
                checkpoint.as_str()
            );
        }
    }
}

/// A read with exactly the submitted fields is the new base: its
/// observation, path, context and source.
#[test]
fn a_read_equal_to_the_save_is_the_new_base() {
    let submitted = document("Saved", "Saved body");
    let previous = base(document("Before", "Old body"));
    let item = read("Saved", "Saved body");
    let after = base_after_save(&submitted, Ok(&item), &previous);
    assert_eq!(after.observation.as_deref(), Some("after"));
    assert_eq!(after.path, "docs/b.md");
    assert_eq!(after.context.kind, ContextKind::Active);
    assert_eq!(after.context.branch.as_deref(), Some("item"));
    assert_eq!(after.context.worktree, "/repo/worktree");
    assert_eq!(after.source.as_deref(), Some("new source"));
    assert!(after.fields == submitted);
}

/// A read that differs from the save, someone else's change, or a read
/// that failed: the base is the submitted fields, unverified.
#[test]
fn a_read_that_differs_or_fails_leaves_the_submitted_fields_unverified() {
    let submitted = document("Saved", "Saved body");
    let previous = base(document("Before", "Old body"));
    let theirs = read("Saved", "Someone else's body");
    for read in [
        Ok(&theirs),
        Err(ResultCode::Busy),
        Err(ResultCode::ItemNotFound),
    ] {
        let after = base_after_save(&submitted, read, &previous);
        assert_eq!(after.observation, None);
        assert!(after.fields == submitted);
        assert_eq!(after.path, previous.path);
        assert_eq!(after.context, previous.context);
    }
}

/// Ruling R13: a relationship equal to the base's is left as the file has
/// it, so an entry the library ignored survives; an emptied one is
/// cleared; any other is set.
#[test]
fn relationships_are_unchanged_when_equal_cleared_when_emptied_and_set_otherwise() {
    let none: [String; 0] = [];
    let one = [OTHER.to_owned()];
    assert_eq!(deps_write(&one, &one), RelationshipWrite::Unchanged);
    assert_eq!(deps_write(&none, &none), RelationshipWrite::Unchanged);
    assert_eq!(deps_write(&none, &one), RelationshipWrite::Clear);
    assert_eq!(
        deps_write(&one, &none),
        RelationshipWrite::Set(vec![OTHER.to_owned()])
    );
    let reordered = [ITEM.to_owned(), OTHER.to_owned()];
    let other_order = [OTHER.to_owned(), ITEM.to_owned()];
    assert_eq!(
        deps_write(&reordered, &other_order),
        RelationshipWrite::Set(reordered.to_vec())
    );

    let parent = OTHER.to_owned();
    assert_eq!(
        parent_write(Some(&parent), Some(&parent)),
        RelationshipWrite::Unchanged
    );
    assert_eq!(parent_write(None, None), RelationshipWrite::Unchanged);
    assert_eq!(parent_write(None, Some(&parent)), RelationshipWrite::Clear);
    assert_eq!(
        parent_write(Some(&parent), None),
        RelationshipWrite::Set(OTHER.to_owned())
    );
}

fn target(kind: ItemKind) -> DraftTarget {
    DraftTarget {
        repository: "/repo".to_owned(),
        item_kind: kind,
        item_id: ITEM.parse().unwrap(),
        comment: None,
    }
}

/// A ticket save carries every field of the draft, its base's observation
/// and the relationship writes of R13.
#[test]
fn a_ticket_save_carries_its_fields_its_observation_and_its_relationship_writes() {
    let base = base(ticket(&[OTHER], Some(OTHER)));
    let mine = ticket(&[OTHER], None);
    let Some(Intent::TicketSave {
        root,
        id,
        title,
        ticket_type,
        status,
        project,
        team,
        body,
        deps,
        parent,
        observation,
    }) = save_intent(&target(ItemKind::Ticket), &mine, &base)
    else {
        panic!("a ticket save");
    };
    assert_eq!(root, PathBuf::from("/repo"));
    assert_eq!(id.to_string(), ITEM);
    assert_eq!(
        (title, ticket_type, status, project, team, body),
        (
            "Title".to_owned(),
            "task".to_owned(),
            "open".to_owned(),
            Some("Project".to_owned()),
            None,
            "Body".to_owned()
        )
    );
    assert_eq!(deps, RelationshipWrite::Unchanged);
    assert_eq!(parent, RelationshipWrite::Clear);
    assert_eq!(observation, "before");
}

#[test]
fn a_document_save_carries_its_title_body_and_observation() {
    let base = base(document("Before", "Old"));
    let Some(Intent::DocumentSave {
        root,
        id,
        title,
        body,
        observation,
    }) = save_intent(&target(ItemKind::Document), &document("New", "Body"), &base)
    else {
        panic!("a document save");
    };
    assert_eq!(root, PathBuf::from("/repo"));
    assert_eq!(id.to_string(), ITEM);
    assert_eq!((title.as_str(), body.as_str()), ("New", "Body"));
    assert_eq!(observation.as_deref(), Some("before"));
}

/// No save is built from an unverified base, for a comment, or for fields
/// of another kind than the target's.
#[test]
fn no_save_is_built_without_an_observation_or_for_mismatched_fields() {
    let mut unverified = base(document("Before", "Old"));
    unverified.observation = None;
    assert!(
        save_intent(
            &target(ItemKind::Document),
            &document("New", "Body"),
            &unverified
        )
        .is_none()
    );
    let verified = base(document("Before", "Old"));
    assert!(
        save_intent(
            &target(ItemKind::Ticket),
            &document("New", "Body"),
            &verified
        )
        .is_none()
    );
    let mut comment = target(ItemKind::Ticket);
    comment.comment = Some(manyhands::drafts::CommentTarget {
        comment_id: OTHER.parse().unwrap(),
        parent_id: None,
    });
    let body = DraftFields::Comment {
        body: "Hi".to_owned(),
    };
    assert!(save_intent(&comment, &body, &verified).is_none());
}
