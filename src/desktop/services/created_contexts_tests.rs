//! `CreatedContexts` takes a context in only from a `context prepare` that
//! answered `ok` with `created: true` (R-75, R-77), and takes it out for a
//! save or a new item's create that committed into it, or that the library
//! found already applied (Part D fix 1, M1).

use std::path::PathBuf;

use manyhands::{
    repository::{
        AuthoringKind, ContextMutationDto, ContextPrepareIntent, ItemDtoKind, MutationDataDto,
        RelationshipWrite,
    },
    results::{CheckpointEffect, Effects, Envelope, Outcome, ResultCode, Scope},
};

use super::{ContextKey, CreatedContexts};
use crate::desktop::{services::requests::Intent, test_support::fixtures::item_id};

const ROOT: &str = "/work/a";
const TICKET: &str = "01ARZ3NDEKTSV4RRFFQ69G5FF0";

fn prepare() -> Intent {
    Intent::ContextPrepare {
        root: PathBuf::from(ROOT),
        kind: AuthoringKind::Ticket,
        id: item_id(TICKET),
        intent: ContextPrepareIntent::Edit,
    }
}

fn answer(outcome: Outcome, code: ResultCode, created: Option<bool>) -> Envelope<MutationDataDto> {
    Envelope {
        schema_version: 1,
        command: "context prepare".to_owned(),
        request_id: None,
        operation_id: None,
        outcome,
        code,
        message: String::new(),
        scope: Scope::default(),
        effects: Effects::not_requested(),
        data: created.map(|created| {
            MutationDataDto::Context(ContextMutationDto {
                id: TICKET.to_owned(),
                kind: "ticket".to_owned(),
                branch: format!("manyhands/ticket/{TICKET}"),
                worktree: format!("{ROOT}/.manyhands/worktrees/{TICKET}"),
                created,
            })
        }),
        recovery: Vec::new(),
    }
}

fn key() -> ContextKey {
    ContextKey::new(ROOT, ItemDtoKind::Ticket, TICKET)
}

#[test]
fn only_an_ok_answer_that_made_the_context_enters_the_set() {
    let mut created = CreatedContexts::default();
    created.finished(
        &prepare(),
        &answer(Outcome::Success, ResultCode::Ok, Some(false)),
    );
    assert!(!created.contains(&key()), "a context found, not made");
    created.finished(
        &prepare(),
        &answer(Outcome::Partial, ResultCode::DiscoveryPending, Some(true)),
    );
    assert!(!created.contains(&key()), "a hand-off still owed (R-77)");
    created.finished(
        &prepare(),
        &answer(Outcome::Blocked, ResultCode::WorktreeNotClean, None),
    );
    assert!(!created.contains(&key()), "a refusal");
    created.finished(
        &Intent::IndexRefresh {
            root: PathBuf::from(ROOT),
        },
        &answer(Outcome::Success, ResultCode::Ok, Some(true)),
    );
    assert!(!created.contains(&key()), "another command");

    created.finished(
        &prepare(),
        &answer(Outcome::Success, ResultCode::Ok, Some(true)),
    );
    assert!(created.contains(&key()));
    assert!(!created.contains(&ContextKey::new(ROOT, ItemDtoKind::Document, TICKET)));
    assert!(created.remove(&key()));
    assert!(!created.contains(&key()));
}

/// The ticket's save and its create, as a session sends them.
fn writes() -> [Intent; 2] {
    let (root, id) = (PathBuf::from(ROOT), item_id(TICKET));
    [
        Intent::TicketSave {
            root: root.clone(),
            id: id.clone(),
            title: "Title".to_owned(),
            ticket_type: "task".to_owned(),
            status: "open".to_owned(),
            project: None,
            team: None,
            body: String::new(),
            deps: RelationshipWrite::Unchanged,
            parent: RelationshipWrite::Unchanged,
            observation: "observed".to_owned(),
        },
        Intent::TicketCreate {
            root,
            id,
            title: "Title".to_owned(),
            ticket_type: "task".to_owned(),
            status: "open".to_owned(),
            project: None,
            team: None,
            body: String::new(),
            deps: Vec::new(),
            parent: None,
        },
    ]
}

/// Part D fix 1, M1: a save or a new item's create answered
/// `already_applied` takes its item's context out, as one that committed
/// does: the context holds the item's commit, and its release would be
/// refused. A refusal leaves it in.
#[test]
fn a_write_committed_or_already_applied_takes_the_context_out() {
    let made = answer(Outcome::Success, ResultCode::Ok, Some(true));
    let mut committed = answer(Outcome::Success, ResultCode::Ok, None);
    committed.effects.checkpoint = CheckpointEffect::Committed;
    for write in writes() {
        let mut created = CreatedContexts::default();
        created.finished(&prepare(), &made);
        created.finished(
            &write,
            &answer(Outcome::Blocked, ResultCode::OccupiedPath, None),
        );
        assert!(created.contains(&key()), "a refusal leaves it in");
        created.finished(
            &write,
            &answer(Outcome::Noop, ResultCode::AlreadyApplied, None),
        );
        assert!(!created.contains(&key()), "already applied takes it out");

        created.finished(&prepare(), &made);
        created.finished(&write, &committed);
        assert!(!created.contains(&key()), "a commit takes it out");
    }
}

/// Task 27 fix 1, B6: a document's move is a write into its context, as a
/// save is. One that committed, or that the library found already applied,
/// takes a context this process made out: it holds the move's commit now,
/// so it is kept, and no release is sent for it. A refusal leaves it in;
/// while one is in flight, a release waits for it (`saved_by`).
#[test]
fn a_document_move_committed_takes_the_context_out() {
    let (root, id) = (PathBuf::from(ROOT), item_id(TICKET));
    let prepare = Intent::ContextPrepare {
        root: root.clone(),
        kind: AuthoringKind::Document,
        id: id.clone(),
        intent: ContextPrepareIntent::Edit,
    };
    let moved = Intent::DocumentMove {
        root,
        id,
        observation: Some("observed".to_owned()),
        destination_path: PathBuf::from("docs/moved.md"),
        destination_observation: "absent".to_owned(),
    };
    let document = ContextKey::new(ROOT, ItemDtoKind::Document, TICKET);
    assert_eq!(ContextKey::of(&moved), Some(document.clone()));
    assert!(document.saved_by(&moved), "a release waits for it");
    assert!(!key().saved_by(&moved), "only its own document's");

    let made = answer(Outcome::Success, ResultCode::Ok, Some(true));
    let mut committed = answer(Outcome::Success, ResultCode::Ok, None);
    committed.effects.checkpoint = CheckpointEffect::Committed;
    let mut created = CreatedContexts::default();
    created.finished(&prepare, &made);
    assert!(created.contains(&document));
    created.finished(
        &moved,
        &answer(Outcome::Blocked, ResultCode::ExternalChange, None),
    );
    assert!(created.contains(&document), "a refusal leaves it in");
    created.finished(&moved, &committed);
    assert!(!created.contains(&document), "a commit takes it out");

    created.finished(&prepare, &made);
    created.finished(
        &moved,
        &answer(Outcome::Noop, ResultCode::AlreadyApplied, None),
    );
    assert!(!created.contains(&document), "already applied takes it out");
}
